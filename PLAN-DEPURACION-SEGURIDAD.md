# Plan de depuración — Soplos Welcome 3.0

Origen: auditoría de seguridad realizada con Claude (sesión externa) sobre el estado del proyecto a 2026-09-16, con lectura completa del backend Rust (`src-tauri/src/**`), la capa de estado/API del frontend (`api.ts`, `progress.ts`, `theme.ts`, `App.svelte`) y las vistas `Recommended.svelte`, `Kernels.svelte`, `Drivers.svelte`, `Software.svelte`, `UpdateActions.svelte`, `TerminalOutput.svelte`.

No cubierto en esa pasada (pendiente de revisión): `Gaming.svelte`, `Customization.svelte`, `Security.svelte`, `SoftwareXfce/Gnome/Plasma.svelte`, `Home.svelte`, `Sidebar.svelte`.

Cada punto crítico fue verificado línea por línea contra el código real antes de anotarse aquí.

## Fase 1 — Crítico (seguridad, ejecución como root)

### 1. Ejecutor de shell genérico expuesto al webview
`src-tauri/src/commands/gaming.rs:44-55`

```rust
pub fn run_user_script(script: String) -> u64 { start_job("bash", vec!["-c", script]) }
pub fn run_privileged_script(script: String) -> u64 { start_pkexec_job(script) }
```

Cualquier JS ejecutado en el webview (incluida la consola de devtools) puede invocar `invoke("run_privileged_script", {script: "..."})` y obtener una shell root sin pasar por ninguna confirmación de la UI. Hoy solo lo llama `Recommended.svelte` con strings fijos en el propio código fuente, así que no hay explotación remota activa — pero la función en sí no valida nada: es una puerta sin cerradura.

**Acción (hecha):** `run_user_script`/`run_privileged_script` eliminados de `gaming.rs`/`lib.rs`. Creado `src-tauri/src/commands/recommended.rs` con 25 comandos específicos (uno por app custom, más `link_ppsspp_lutris_runner`), cada uno con su script fijo en Rust. `Recommended.svelte` y `Gaming.svelte` reescritos para llamar a esos comandos por nombre (`customInstall`/`customUninstall`), sin mandar texto de shell desde el frontend. Verificado: `svelte-check` 0 errores/0 avisos, `cargo check` limpio.

### 2. `devtools` activo incondicionalmente en release
`src-tauri/Cargo.toml:15`

```toml
tauri = { version = "2", features = ["devtools"] }
```

No está condicionado a `debug_assertions`. Combinado con el punto 1, cualquiera con acceso local al binario final puede abrir el inspector y ejecutar `window.__TAURI__.core.invoke("run_privileged_script", {...})` directamente desde la consola.

**Acción:** mover `devtools` a un perfil/feature condicional de forma que el build de producción no lo incluya.

### 3. Interpolación de strings sin sanitizar hacia `pkexec sh -c`
Patrón repetido en `software.rs`, `drivers.rs`, `kernels.rs`, `security.rs`, `gaming.rs`. Ejemplos verificados:

- `install_apt_packages(packages: String)` → `format!("apt install -y {packages}")` (`software.rs:257-259`)
- `repair_wifi(iface, driver: String)` → interpolado sin comillas en `modprobe {driver}` (`drivers.rs:686-700`)
- `install_snap`, `install_soplos_kernel`, `uninstall_soplos_kernel`, `install_rocm`, etc.

Hoy todos los llamadores del frontend pasan valores controlados (nombres derivados de `dpkg`/`apt-cache`), así que la explotación práctica es baja ahora mismo. Es un patrón "seguro por convención, no por construcción": el día que un campo de texto libre llegue a uno de estos comandos, hay inyección de comandos como root.

**Acción (hecha):** añadidos 4 validadores reutilizables en `src-tauri/src/utils/mod.rs` (`is_valid_pkg_name`, `is_valid_pkg_list`, `is_valid_flatpak_id`, `is_valid_identifier`). Aplicados antes de construir el script en `install_apt_packages`/`uninstall_apt_packages`/`install_snap`/`uninstall_snap`/`install_flatpak_scoped`/`uninstall_flatpak_scoped` (software.rs), `install_nvidia_repo`/`install_nvidia_cuda`/`repair_wifi` (drivers.rs), `install_soplos_kernel`/`uninstall_soplos_kernel`/`update_soplos_kernel`/`clean_kernels` (kernels.rs, usando también `is_valid_soplos_pkg_name` que ya existía sin usar) e `install_deb_url` (gaming.rs — se me escapó en la primera pasada, encontrado al revisar `format!` en el resto de archivos antes de dar el punto por cerrado; se añadió un quinto validador, `is_valid_https_url`). Cada función pasó de devolver `u64` a `Result<u64, String>`, rechazando con error claro cualquier valor con formato inesperado — sin cambios necesarios en el frontend porque `runTracked` ya capturaba errores de `invoke()`. `install_rocm` no necesitaba cambio (el parámetro `mode` nunca se interpola, solo se compara). `install_xanmod`/`uninstall_xanmod` tampoco (el `package` ya sale siempre de una función que valida contra una lista cerrada). Verificado: `cargo check` sin avisos, `svelte-check` 0/0.

### 4. `privilege/mod.rs` y `services/mod.rs` son stubs que documentan una arquitectura inexistente

```rust
// pkexec/polkit helpers for privileged operations.
// The UI process never runs as root; only scoped helper binaries do,
// invoked through pkexec with a restrictive polkit policy.
```

No existe ningún archivo `.policy` de polkit en el repo (búsqueda explícita, sin resultados). Lo que hay realmente es `pkexec sh -c <script>`, que en Debian pide autenticación de administrador completa, no una acción polkit acotada.

**Acción (hecha):** implementado un modelo de polkit real, reutilizando el propio binario en vez de crear uno auxiliar nuevo:
- `resources/polkit/org.soploslinux.welcome.policy` declara la acción `org.soploslinux.welcome.run-privileged`, con `org.freedesktop.policykit.exec.path` apuntando a `/usr/bin/soplos-welcome` — así polkit autoriza específicamente a este binario, no a `/bin/sh` en general (que usa todo el sistema).
- `main.rs` intercepta el flag interno `--run-privileged-script <path>` antes de arrancar Tauri; `privilege::run_privileged_script_file` (en `privilege/mod.rs`, que ahora sí contiene código real) lee el script del archivo, lo borra y lo ejecuta con `sh -c`, heredando la entrada/salida para que el streaming en vivo del terminal siga funcionando igual que antes.
- `software.rs`: `start_pkexec_job` y `start_confirmable_pkexec_job` ahora escriben el script a un archivo temporal privado (modo 600, en `/tmp`, borrado tras leerse) y lanzan `pkexec <ruta-del-propio-binario> --run-privileged-script <archivo>` en vez de `pkexec sh -c "<script>"` directo.
- `tauri.conf.json`: `bundle.linux.deb.files` registra el `.policy` para que se instale en `/usr/share/polkit-1/actions/` al generar el `.deb`.

Verificado: `cargo check` limpio. **No verificado**: no se ha generado un `.deb` real todavía (a petición explícita), así que la ruta de instalación del `.policy` y el comportamiento de polkit en un sistema real siguen sin probarse en vivo — solo por lectura de la documentación de Tauri y del propio código fuente de la crate `tauri`.

## Fase 2 — Alto (concurrencia / UX)

### 5. Stores de progreso globales — dos operaciones root simultáneas se pisan
`src/lib/progress.ts`

`isRunning`, `terminalLines`, `progressFraction`, `terminalExpanded` son stores Svelte a nivel de módulo, compartidos por toda la app, no por operación. Nada impide lanzar una segunda operación mientras la primera corre (cada vista solo tiene su propio flag `busy` local). Ambos jobs corren en threads Rust independientes correctamente, pero ambos escriben al mismo terminal/progreso compartido del frontend — el usuario vería una mezcla de la salida de dos procesos root distintos, sin forma de saber cuál terminó con éxito.

**Acción (hecha):** se optó por la guardia mínima, no el rediseño por-job. `runTracked`/`runConfirmableUpdate` en `progress.ts` comprueban `get(isRunning)` al entrar y devuelven `false` sin tocar ningún store si ya hay un job corriendo — así la operación en curso sigue mostrando su salida sin que una segunda acción la pise. Verificado: `svelte-check` 0/0.

## Fase 3 — Medio / Bajo

### 6. `JOBS`/`JOB_STDIN` nunca purgan entradas viejas
`software.rs:232-245`. El `HashMap<u64, JobStatus>` con `output: String` acumulado por cada job nunca se libera — fuga de memoria menor durante la vida del proceso.

**Acción (hecha):** `poll_job` ahora elimina la entrada del `HashMap` en el momento en que devuelve un estado con `running: false` — que es siempre la última vez que el frontend lo consulta (`runTracked`/`runConfirmableUpdate` dejan de sondear en cuanto ven `running: false`). Verificado: `cargo check` limpio.

### 7. `confirm_pending_job` con stdin ya removido — ya cerrado
Si `pkexec` termina justo antes de que el frontend llame a `confirm_pending_job`, la función devuelve `Err(...)`, que se propaga correctamente al catch de `runConfirmableUpdate` en `progress.ts:135`. Verificado como manejado correctamente — sin acción.

### 8. `estimateFraction` puede leer porcentajes falsos
`src/lib/progress.ts`. El regex `PERCENT_PATTERN = /(\d{1,3})%/g` toma el último porcentaje visto en todo el output acumulado, sin distinguir barras de progreso `\r`-animadas de cualquier otro número seguido de `%` en el log (ej. "CPU usage 12%"). Cosmético, no funcional, pero puede confundir sobre si algo se ha colgado.

**Acción (hecha):** `PERCENT_PATTERN` (que buscaba "N%" en cualquier parte del output acumulado) sustituido por `PROGRESS_LINE_PATTERN`, que solo reconoce líneas con pinta real de barra de progreso: wget ("  45%[...]", porcentaje al principio de línea) o dpkg/apt/flatpak ("Progress: [ 45%]"/"Progreso: [ 45%]"). `estimateFraction` recorre las líneas de la más reciente a la más antigua y usa la primera que encaje, en vez de escanear todo el texto en busca de cualquier "N%" suelto. Verificado: `svelte-check` 0/0.

### 9. Descargas de terceros sin verificación de checksum/firma
Los flujos `custom` de `Recommended.svelte` (Stacer, ES-DE, Affinity, CoolerControl, Double Commander) descargan directo a `~/AppImages/` confiando solo en HTTPS + URL hardcodeada. El `.desktop` de CoolerControl usa `Exec=pkexec .../CoolerControlD-x86_64.AppImage` — se ejecuta como root sin auditar el binario.

**Acción (hecha):** implementada la opción 3 (verificar donde el proveedor publique un checksum, sin bloquear donde no). `utils::VERIFY_DOWNLOAD_SH` define una función de shell `verify_and_download()` que descarga el archivo y, si existe un `<url>.sha256` publicado junto a él (convención habitual de GitHub/GitLab releases), lo verifica antes de continuar — si la verificación falla, aborta la instalación; si el proveedor no publica checksum, instala igual que antes, sin romper nada. Aplicado en las 12 descargas directas de `recommended.rs`, en `install_deb_url` (`gaming.rs`) y en `install_stacer`/`install_portmaster`/`install_kudu` (`security.rs` — estos dos últimos no estaban en el barrido original, se encontraron al revisar el patrón). No se tocaron las descargas de claves GPG (VS Code, Sublime Text) porque no tiene sentido comprobar un checksum de la propia clave de confianza. Verificado: `cargo check` sin avisos.

### 10. Residuo de plantilla cross-platform
`src-tauri/src/main.rs:2` — `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`. Inofensivo en Linux pero puede confundir: todo el backend real usa `pkexec`, `dpkg`, `/proc`, `/sys`, `apt` — no hay soporte Windows.

**Acción (hecha):** eliminada la línea `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` de `main.rs`. Verificado: `cargo check` limpio.

### 11. Sin tests
No hay ningún `#[test]` en el backend Rust ni tests de frontend. Para una app que ejecuta comandos root sobre gestión de kernels/drivers/GRUB, la única red de seguridad actual es la revisión manual.

### 12. No hay instancia única
No existe `tauri-plugin-single-instance` ni ninguna comprobación equivalente en `lib.rs`. Nada impide abrir varias ventanas de Soplos Welcome a la vez, cada una con su propio job runner y estado de progreso independiente — varias pudiendo lanzar `pkexec` en paralelo sobre el mismo sistema sin coordinación entre ellas. Agrava directamente el punto 5.

**Acción:** añadir `tauri-plugin-single-instance` para que al intentar abrir una segunda instancia, se enfoque la ventana ya existente en vez de crear una nueva.

### 13. `apt autoremove` sin filtrar puede arrastrar paquetes no relacionados (hallazgo en vivo, no de la auditoría original)
Reportado por el usuario tras una desinstalación real de kernel: al desinstalar `linux-soplos-rt-v4`, `uninstall_soplos_kernel` ejecutaba `apt autoremove --purge -y` sin ningún filtro justo después de quitar el kernel. `apt autoremove` limpia **cualquier** paquete del sistema marcado como "instalado automáticamente y ya no necesario", no solo lo relacionado con la acción que lo disparó — en este caso se llevó por delante ClamAV completo, todos los drivers de impresora y todo Ruby (50 paquetes), sin avisar ni pedir confirmación.

**Acción (hecha):** quitado el `apt autoremove --purge -y` de `uninstall_soplos_kernel` (`kernels.rs`) — el kernel y sus paquetes asociados ya se eliminan explícitamente con `apt remove -y {package} $EXTRA`, no hace falta un autoremove global. Verificado: `cargo check` limpio.

**Acción (hecha):** quitado también el mismo `apt autoremove -y`/`apt autoremove --purge -y` de las 7 ocurrencias en `drivers.rs` (`install_nvidia_repo`, `install_nvidia_cuda`, `uninstall_nvidia`, `uninstall_oneapi`, `uninstall_cuda12`, `uninstall_rocm`, `uninstall_vbox_guest`) y de `clean_kernels` (`kernels.rs`). Principio aplicado: ninguna acción de instalar/desinstalar debe disparar una limpieza de paquetes huérfanos en todo el sistema como efecto secundario — cada acción quita exactamente lo que dice que quita, nada más. Se conserva `clean_system()` en `software.rs`, que sí es un botón dedicado y explícito a limpiar el sistema (no un efecto secundario de otra acción). Verificado: `cargo check` limpio.

### 14. Menú contextual de WebKitGTK visible en producción (hallazgo posterior, no de la auditoría original)
Al verificar el punto 2 en el binario de release real se vio que, aunque el inspector ya no aparece, el clic derecho seguía mostrando el menú de navegador (Atrás/Adelante/Detener/Recargar) — impropio de una app de escritorio nativa.

**Acción (hecha):** `src/main.ts` desactiva `contextmenu` globalmente cuando `!import.meta.env.DEV`, dejándolo intacto en dev para poder seguir abriendo el inspector con clic derecho durante el desarrollo.

---

## Checklist de reparación

- [x] 1. Eliminar `run_user_script`/`run_privileged_script`, sustituir por comandos Rust acotados por app en Recommended y Gaming
- [x] 2. Condicionar `devtools` a `debug_assertions`, fuera del build de release — resuelto quitando el feature del Cargo.toml (Tauri ya lo activa solo en debug builds internamente)
- [x] 3. Sanear interpolación hacia `pkexec sh -c` en software.rs/drivers.rs/kernels.rs/gaming.rs (security.rs no tenía puntos de interpolación sin validar — sus `format!` solo usan `whoami`/`$HOME`, nunca parámetros del frontend)
- [x] 4a. Corregir el comentario engañoso de `privilege/mod.rs`
- [x] 4b. Implementar polkit real — hecho reutilizando el propio binario con un modo `--run-privileged-script`, verificado por compilación; pendiente de probar con un `.deb` real
- [x] 5. Guard mínimo contra jobs simultáneos en `runTracked`/`runConfirmableUpdate`
- [x] 6. Purgar `JOBS`/`JOB_STDIN` al terminar cada job
- [x] 7. (Ya cerrado desde el análisis inicial — sin acción necesaria)
- [x] 8. Restringir `estimateFraction` a líneas de progreso real
- [x] 9. Verificación de checksum (donde el proveedor lo publique) en descargas de terceros
- [x] 10. Quitar el residuo `windows_subsystem` de `main.rs`
- [x] 11. Añadir tests mínimos al backend Rust (validadores de shell-safety) — 8 tests en `utils/mod.rs` y `kernels.rs`, `cargo test` en verde
- [x] 12. Añadir `tauri-plugin-single-instance` para impedir varias ventanas simultáneas — registrado como primer plugin en `lib.rs`, enfoca la ventana existente en vez de abrir una nueva. Verificado: `cargo check` limpio.
- [x] 13a. Quitar el `apt autoremove --purge -y` sin filtrar de `uninstall_soplos_kernel`
- [x] 13b. Quitar el mismo patrón de las 7 funciones de `drivers.rs` y de `clean_kernels`
- [x] 14. Desactivar el menú contextual de WebKitGTK en producción (`main.ts`, gated a `!import.meta.env.DEV`)
- [x] 15. Arreglar `printf` roto en los `.desktop` de AppImage
- [x] 16. Icono real + integración AppImage Manager + WebApp de CoolerControl
- [x] 17. Persistencia de tema claro/oscuro — `theme.ts` creaba el store con "dark" y disparaba `subscribe(applyTheme)` (que también escribe en localStorage) antes de leer la preferencia guardada, sobrescribiéndola con "dark" en cada arranque. Corregido: el store ahora se crea directamente con `readStoredTheme()`. Verificado: `svelte-check` 0/0, pendiente de que el usuario confirme tras un cierre/reapertura completos.
- [ ] 18. Hay que agregar tarjetas informativas de hardware especifico en la pestaña gaming.

### 15. `printf` roto en los `.desktop` generados por AppImage (bug real encontrado en vivo, mío, de antes de esta sesión de arreglos)
Al probar Recommended en la app real, Double Commander (y por el mismo patrón: Affinity, ES-DE, CoolerControl) fallaban con `bash: printf: 'H': carácter de formato inválido`. Causa: `printf '{CONST}'` pasaba el texto del `.desktop` (que contiene el marcador `%HOME%`) como **formato** de `printf`, y `printf` interpreta cualquier `%X` de su propio formato como especificador — `%H` no es válido, y aborta. Llevaba roto desde que se escribieron estas funciones esta misma sesión (parte del punto 1), sin haberse probado en vivo hasta ahora.

**Acción (hecha):** cambiado `printf '{CONST}'` por `printf '%s' '{CONST}'` en las 4 funciones (`install_affinity`, `install_esde`, `install_coolercontrol`, `install_doublecmd`) — así el texto viaja como **argumento** de `printf`, no como formato, y su contenido nunca se interpreta. Verificado con una simulación exacta del pipeline bash generado (no solo `cargo check`): el `.desktop` se genera bien, con saltos de línea reales y `%HOME%` sustituido correctamente.

### 16. Instaladores AppImage (Affinity, ES-DE, CoolerControl, Double Commander) sin icono real y sin integración con AppImage Manager (bug real de esta sesión, encontrado en vivo por el usuario)
Al probar Double Commander en la app, no se instalaba visiblemente nada (ni terminal, ni icono) aunque la instalación sí ocurría por debajo. Comparando con `soplos-welcome` 2.x y `soplos-appimage-manager` se encontraron 4 carencias en el port de estos 4 instaladores:
1. Icono genérico de tema (`Icon=system-file-manager`) en vez de copiar el PNG real del programa a `~/AppImages/.icons/`, como hace 2.x.
2. Faltaba `X-AppImage-Integrate=true` — la marca que usa `soplos-appimage-manager` (confirmado leyendo su código) para reconocer estas entradas como gestionadas por él.
3. Faltaba `update-desktop-database` tras crear el `.desktop` — necesario para refrescar la caché de iconos/menú (también confirmado en el código de AppImage Manager).
4. Los scripts no tenían ningún `echo`, así que `terminalLines` se quedaba vacío y el panel de terminal nunca se mostraba — no era una cuestión de velocidad, es que no había nada que mostrar.
5. (Solo CoolerControl) faltaba por completo la segunda entrada "WebApp" que abre su interfaz web (`localhost:11987`) en un navegador en modo app, con detección automática de navegador — 2.x sí la crea, el port inicial no.

**Acción (hecha):** los 4 iconos PNG se copiaron a `src-tauri/resources/appimage-icons/` y se registraron en `tauri.conf.json`. Las 4 funciones `install_*` ahora reciben `tauri::AppHandle`, resuelven el icono como recurso de Tauri, lo copian a `~/AppImages/.icons/`, añaden `X-AppImage-Integrate=true`, llaman a `update-desktop-database` y tienen mensajes `echo` de progreso — igual en las 4 funciones `uninstall_*`. CoolerControl se reescribió completo para portar también la entrada WebApp con detección de navegador (Chromium/Brave/Chrome/Vivaldi → Firefox → flatpak → `xdg-open`), verbatim del script de 2.x.

**Verificado de verdad, no solo compilado:** se ejecutaron en vivo (con descarga real) los scripts completos de Double Commander y CoolerControl — ambos crean el `.desktop` correcto con icono real, y CoolerControl además genera bien la entrada WebApp con el navegador detectado (Google Chrome, en este caso) y descarga el AppImage real (42 MB). Affinity y ES-DE siguen el mismo patrón ya corregido pero no se ejecutaron individualmente en vivo — no se puede afirmar lo mismo de esos dos con la misma certeza.

### 17. Persistencia de tema claro/oscuro
El store se creaba con "dark" y `subscribe(applyTheme)` escribía esa "dark" en localStorage antes de leer la preferencia guardada. **Hecho:** el store se crea directamente con `readStoredTheme()`. `svelte-check` 0/0; pendiente de que el usuario confirme tras un cierre/reapertura completos.

### 18. Tarjetas de hardware en Gaming
Añadidas 4 tarjetas (CPU, GPU, Memoria, Optimizaciones activas) entre el hero y los desplegables, mismo estilo que Drivers, reutilizando `scan_hardware`. **Hecho.** `svelte-check` 0/0, sin cambios en Rust.

### 19. Nuevas categorías en Recomendados: Remote Desktops y Virtualization
- **Remote Desktops** (4 apps, todas flatpak tras corrección a petición del usuario): Remmina (`org.remmina.Remmina`), RustDesk (`com.rustdesk.RustDesk`), AnyDesk (`com.anydesk.Anydesk`), NoMachine (`com.nomachine.nxplayer` — el cliente, no el servidor). Las 4 verificadas contra la API de Flathub antes de añadirlas.
- **Virtualization**: virt-manager y GNOME Boxes por apt (confirmados en Debian). **VirtualBox** no está en los repos de Soplos (confirmado con `apt-cache`) — se añadió el repositorio oficial de Oracle, con detección del codename Debian y el paquete versionado correcto (`virtualbox-7.2`); corregido tras aviso del usuario un fallo real (faltaba `gpg --dearmor` de la clave, comparado con la documentación oficial de Oracle que pegó).
- **VirtualBox Extension Pack**: añadido con diálogo de aceptación de licencia PUEL propio (sin tocar el flujo de DaVinci). El hash de la licencia se calculó de verdad extrayendo el archivo real del Extension Pack (`sha256sum` de `ExtPack-license.txt`), no de memoria. La versión del pack se detecta en tiempo real contra la versión de VirtualBox instalada.

**Sin verificar en vivo:** ninguna de las apps de esta sección se ha instalado de verdad todavía (a diferencia de los AppImage del punto 16). La ruta de `checkPath` del Extension Pack (`/usr/lib/virtualbox/ExtensionPacks/Oracle_VirtualBox_Extension_Pack`) es una suposición sin confirmar — no hay VirtualBox instalado en esta máquina para comprobar el nombre real de la carpeta.

## Estado

Resueltos: 1, 2, 3, 4a, 4b, 5, 6, 7, 8, 9, 10, 11, 12, 13a, 13b, 14, 15, 16, 17, 18, 19. **No queda ningún punto pendiente del plan de seguridad original ni de lo añadido hoy.**

Puntos verificados en vivo, no solo por compilación: 2, 4b (parcial, falta probar con `.deb` real), 11 (`cargo test` en verde), 13a/13b (a partir del log real de desinstalar un kernel), 14, 15 y 16 (Double Commander y CoolerControl ejecutados de verdad; Affinity y ES-DE no probados individualmente). El resto (1, 3, 5, 6, 8, 9, 10, 12, 17, 18, 19) solo están verificados por compilación, no probados en vivo todavía.

### 20. Bugs reales encontrados probando en vivo el catálogo nuevo
- **`verify_and_download` daba falsos positivos de "checksum inválido"** — GitLab (y probablemente otros proveedores) no devuelve 404 al pedir `<url>.sha256` si no existe, re-sirve el archivo original entero. La comprobación solo miraba "no está vacío", así que se tragaba el binario de 129MB como si fuera un hash y fallaba al verificarlo — abortando instalaciones que en realidad estaban bien (ES-DE). **Hecho:** ahora exige que el "checksum" pese ≤1024 bytes y tenga pinta de hash hexadecimal de 64 caracteres antes de confiar en él. Verificado de verdad contra la URL real de ES-DE (antes fallaba, ahora descarga los 129MB correctos). Afecta a las ~14 descargas que usan esta función compartida, no solo a ES-DE.
- **VirtualBox no instalaba — causa real encontrada y arreglada, no era una incompatibilidad sin solución.** El build de Oracle para trixie necesita `libvpx9`, que ya no está en los repos base de forky (testing) — pero **sí sigue existiendo en `trixie-security`**, un repo que Soplos no tiene activado por defecto. Confirmado a mano en esta misma máquina: en cuanto se activa `trixie-security`, `apt install virtualbox-7.2` resuelve *todo* limpio (las librerías Qt6/curl/ssl que parecían un problema en el primer intento con `dpkg -i` en realidad ya resolvían bien vía apt — el error de `dpkg -i` solo lista lo que no está instalado *en ese momento*, no lo que es imposible de instalar). Se descartó por el camino la pista de un hilo del foro oficial de VirtualBox (un desarrollador de Oracle confirmando el mismo patrón para virtualbox-7.1/bookworm) y se verificó con el `.deb` real descargado que el build de bookworm cambia un problema por cinco nuevos (GTK2/libcurl4/libssl3 sin t64 en forky) — la vía correcta era añadir `trixie-security`, no cambiar de build. **Hecho:** `install_virtualbox` ahora añade también el repo `trixie-security` (mismo formato DEB822 y `Signed-By` que usa el propio Soplos Repo Selector) antes de `apt update` + `apt install`, sin necesitar ningún `dpkg -i` a la fuerza. `uninstall_virtualbox` limpia ese repo también. Verificado en esta máquina: VirtualBox queda `install ok installed` de verdad.

Notas aparte:
- Se añadió `src/vite-env.d.ts` (faltaba el archivo estándar de tipos de Vite), lo cual también resolvió el error preexistente de `svelte-check` sobre el import de `global.css` que se venía descartando como "ajeno" — no lo era, era un archivo real que faltaba.
- Se corrigió el tipado de `views` en `App.svelte` (`Record<string, unknown>` → `Record<string, ComponentType>`), el otro error que llevaba toda la sesión etiquetado como "preexistente y ajeno" y tampoco lo era. El proyecto compila ahora con 0 errores y 0 avisos en `svelte-check`.
