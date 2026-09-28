# Soplos Welcome 3.0 — Informe Técnico Completo

**Proyecto:** Soplos Welcome  
**Versión objetivo:** 3.0  
**Fecha del informe:** 14 de septiembre de 2026  
**Autor del análisis:** Grok (xAI) + desarrollador Soplos Linux  
**Estado:** Propuesta de arquitectura y plan de implementación  

---

## 1. Resumen ejecutivo

Soplos Welcome es la aplicación central de post-instalación de Soplos Linux. Actualmente está implementada en **Python + GTK3** y ofrece gestión de drivers, kernels, software recomendado, seguridad, personalización y gaming, con soporte multi-DE (XFCE, Plasma y GNOME).

Para la versión **3.0** se propone una reescritura completa con los siguientes objetivos prioritarios:

1. **Interfaz visual moderna** y con personalidad propia de Soplos.
2. **Aspecto 100 % idéntico** en todos los entornos de escritorio y compositors (XFCE, Plasma, GNOME, UKUI, Niri, Sway, Hyprland y futuros).
3. **Mejor rendimiento** (Python se percibe cada vez más lento para este tipo de herramienta).
4. Base técnica sólida y mantenible a largo plazo.
5. Posibilidad de empezar de cero o refactorizar de forma agresiva.

**Decisión recomendada:**  
**Tauri 2 + frontend web (HTML/CSS/JS) + lógica de negocio principalmente en Rust.**

---

## 2. Situación actual de Soplos Welcome (2.x)

### 2.1 Stack técnico actual
- Lenguaje: Python 3
- Toolkit UI: GTK3 (PyGObject)
- Arquitectura: modular
- Estructura principal del repositorio:
  - `core/`
  - `ui/`
  - `services/`
  - `utils/`
  - `config/`
  - `locale/` (8 idiomas)
  - `assets/`
  - `debian/`

### 2.2 Funcionalidades principales
- Detección completa de hardware (CPU, GPU NVIDIA/AMD, Wi-Fi, híbridos PRIME, etc.)
- Gestión avanzada de drivers NVIDIA (series recientes + legacy)
- Gestión de kernels (Liquorix, XanMod y variantes, microcode, limpieza)
- Software (APT, Flatpak, Snap, repositorios propios)
- Catálogo de +60 aplicaciones recomendadas con instalación por lotes
- Centro de seguridad (firewall, backups, antivirus, limpieza)
- Personalización y herramientas exclusivas de Soplos
- Pestaña Gaming
- Escalada de privilegios mediante `pkexec`
- Detección automática de entorno de escritorio y protocolo (X11/Wayland)
- Internacionalización completa (8 idiomas)
- Theming CSS que intenta adaptarse al sistema

### 2.3 Limitaciones actuales
- Aspecto visual limitado por GTK3.
- Diferencias de apariencia entre escritorios.
- GTK4 descartado por mala integración visual en Plasma.
- Rendimiento de Python insuficiente para el nivel de herramienta de sistema que se desea.
- Dificultad para garantizar look idéntico en compositors tiling futuros.

---

## 3. Requisitos de la versión 3.0

### 3.1 Requisitos funcionales
- Mantener (o mejorar) todas las capacidades actuales.
- Interfaz moderna, clara y atractiva.
- Experiencia de usuario consistente.
- Soporte multi-idioma.
- Operaciones privilegiadas seguras.

### 3.2 Requisitos no funcionales (críticos)
- **Look idéntico** en XFCE, Plasma, GNOME, UKUI, Niri, Sway, Hyprland y futuros.
- Buen rendimiento y bajo consumo de recursos.
- Binario/empaquetado razonable para una distribución basada en Debian.
- Código mantenible y escalable.
- Seguridad (principio de mínimo privilegio).

---

## 4. Evaluación de alternativas

| Tecnología              | Look idéntico | Rendimiento | Esfuerzo | Integración sistema | Recomendación |
|-------------------------|---------------|-------------|----------|---------------------|---------------|
| GTK3 (actual)           | No            | Medio       | Bajo     | Buena               | Descartado    |
| GTK4 + libadwaita       | No            | Bueno       | Medio    | Mala en Plasma      | Descartado    |
| Qt6 / PySide6           | No            | Bueno       | Alto     | Excelente en Plasma | Descartado por consistencia visual |
| Python + WebView        | Sí            | Medio       | Bajo-Medio | Buena            | Válida (plan B) |
| **Tauri 2 + Rust**      | **Sí**        | **Excelente** | Medio-Alto | Excelente        | **Recomendada** |
| Electron                | Sí            | Bajo        | Medio    | Media               | No recomendado |

**Conclusión:** Tauri 2 es la opción que mejor cumple el requisito de aspecto idéntico + rendimiento + futuro.

---

## 5. Arquitectura propuesta — Tauri 2

### 5.1 Visión general

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (Web)                          │
│  HTML + CSS + JavaScript (o Svelte / Vue / React)           │
│  Diseño 100 % controlado → look idéntico en todos los DE    │
└──────────────────────────┬──────────────────────────────────┘
                           │ IPC (invoke) — rápido y tipado
┌──────────────────────────▼──────────────────────────────────┐
│                  Tauri Core (Rust)                          │
│  • Gestión de ventana y ciclo de vida                       │
│  • Sistema de permisos (Capabilities)                       │
│  • Comandos de negocio (drivers, kernels, software…)        │
│  • Orquestación de operaciones privilegiadas                │
└──────────────────────────┬──────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                                 ▼
   Comandos del sistema              Workers privilegiados
   (lspci, lscpu, apt…)              (pkexec + polkit)
```

### 5.2 Decisiones de diseño clave

- **UI web controlada al 100 %**: no se depende del tema del escritorio.
- **Lógica principal en Rust**: máximo rendimiento y control.
- **Python solo si aporta valor real** (en la mayoría de casos no será necesario).
- **Separación de privilegios**: la UI nunca corre como root. Las operaciones sensibles se ejecutan mediante `pkexec` / polkit de forma controlada.
- **Modularidad**: cada área funcional (Drivers, Kernels, Recommended, etc.) se implementa como módulo de comandos Rust + componentes de frontend.

### 5.3 Stack tecnológico recomendado

| Capa              | Tecnología recomendada              | Alternativas aceptables      |
|-------------------|-------------------------------------|------------------------------|
| Shell / Runtime   | Tauri 2                             | —                            |
| Backend           | Rust                                | —                            |
| Frontend          | Svelte o Vue 3 + TypeScript         | React, vanilla + Alpine      |
| Estilos           | Tailwind CSS                        | CSS propio / UnoCSS          |
| Iconos            | Lucide / Heroicons / Phosphor       | SVG propios                  |
| Empaquetado       | .deb nativo de Tauri                | AppImage (opcional)          |
| Internacionalización | i18next o similar                | Sistema propio               |
| Build             | Vite + Tauri CLI                    | —                            |

### 5.4 Estructura de proyecto sugerida

```
soplos-welcome-3/
├── src-tauri/                    # Backend Rust
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── system.rs         # Info del sistema
│   │   │   ├── drivers.rs        # NVIDIA, AMD, Wi-Fi…
│   │   │   ├── kernels.rs
│   │   │   ├── software.rs
│   │   │   ├── security.rs
│   │   │   └── gaming.rs
│   │   ├── services/             # Lógica de negocio
│   │   ├── privilege/            # pkexec / polkit helpers
│   │   └── utils/
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                          # Frontend
│   ├── index.html
│   ├── main.ts
│   ├── App.svelte (o App.vue)
│   ├── components/
│   │   ├── Sidebar.svelte
│   │   ├── Header.svelte
│   │   ├── SystemCard.svelte
│   │   ├── AppCard.svelte
│   │   └── ...
│   ├── views/
│   │   ├── Home.svelte
│   │   ├── Drivers.svelte
│   │   ├── Kernels.svelte
│   │   ├── Recommended.svelte
│   │   ├── Security.svelte
│   │   ├── Customization.svelte
│   │   └── Gaming.svelte
│   ├── styles/
│   ├── i18n/
│   └── lib/                      # API wrappers (invoke)
│
├── public/
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 6. Diseño de interfaz (boceto)

Se ha elaborado un boceto HTML/CSS autocontenido que refleja la dirección visual deseada:

- Tema oscuro moderno
- Sidebar con navegación clara
- Hero + tarjetas de información del sistema
- Acciones rápidas con badges de estado
- Listados de drivers con estado visual
- Diseño completamente controlado por CSS (idéntico en cualquier DE)

El boceto se incluye como archivo separado o se puede extraer del historial de conversación.  
**Archivo recomendado:** `mockup-soplos-welcome-3.html`

### Principios de diseño visual
- Tipografía limpia (Inter o sistema)
- Espaciado generoso pero no excesivo
- Jerarquía visual clara
- Badges de estado (éxito / pendiente / advertencia)
- Micro-interacciones suaves
- Modo claro y oscuro propios de Soplos (no dependientes del DE)
- Identidad visual propia (gradientes sutiles, color de acento característico)

---

## 7. Gestión de privilegios (crítico)

### 7.1 Principio
La aplicación **nunca** debe ejecutarse completa como root.

### 7.2 Patrón recomendado
1. Proceso principal (Tauri + WebView) corre como usuario normal.
2. Cuando se necesita una operación privilegiada:
   - Se invoca un comando Rust.
   - Ese comando lanza `pkexec` (o un helper polkit) sobre un binario/script específico y acotado.
3. Se define una policy polkit clara y restrictiva.
4. El frontend solo recibe el resultado (éxito/error + mensaje).

### 7.3 Ejemplo conceptual (Rust)
```rust
#[tauri::command]
async fn install_nvidia_driver(version: String) -> Result<String, String> {
    // Validar versión
    // Construir comando seguro
    // Ejecutar vía pkexec
    // Devolver resultado
}
```

---

## 8. Plan de implementación por fases

### Fase 0 — Preparación (1 semana)
- Crear repositorio nuevo `soplos-welcome-3`
- Configurar Tauri 2 + Vite + Svelte/Vue + Tailwind
- Definir sistema de diseño (colores, tipografía, componentes base)
- Configurar CI básica

### Fase 1 — Prototipo visual + sistema (2-3 semanas)
- Shell de la aplicación (sidebar + routing)
- Vista Home con información del sistema
- Comandos Rust básicos (`get_system_info`, detección de DE/GPU)
- Probar en Plasma, GNOME, XFCE y al menos un compositor tiling

### Fase 2 — Drivers y Kernels (3-4 semanas)
- Portar lógica de detección e instalación de drivers
- Gestión de kernels
- Sistema de privilegios funcional
- UI completa de ambas pestañas

### Fase 3 — Software, Recommended y Security (3-4 semanas)
- Catálogo de aplicaciones
- Instalación por lotes
- Herramientas de seguridad

### Fase 4 — Gaming, Customization y pulido (2-3 semanas)
- Pestaña Gaming
- Personalización
- Traducciones
- Temas claro/oscuro
- Empaquetado .deb final
- Pruebas en múltiples entornos

### Fase 5 — Estabilización y release
- Corrección de bugs
- Documentación
- Release 3.0.0

**Duración estimada total:** 3-5 meses según dedicación.

---

## 9. Empaquetado y distribución

- Formato principal: **.deb** (generado por el bundler de Tauri)
- Dependencias del sistema: WebKitGTK 4.1, etc. (gestionado por Tauri)
- Incluir:
  - Archivo `.desktop`
  - Iconos
  - Policy polkit
  - Posibles helpers privilegiados
- Probar en las tres ediciones actuales de Soplos (Tyron, Tyson, Boro)
- Considerar AppImage como formato secundario opcional

---

## 10. Riesgos y mitigaciones

| Riesgo                              | Probabilidad | Impacto | Mitigación |
|-------------------------------------|--------------|---------|----------|
| Problemas de renderizado WebKitGTK en Sway/Hyprland | Media       | Medio   | Probar temprano, reportar bugs, tener fallbacks |
| Curva de aprendizaje de Rust        | Alta         | Medio   | Empezar con comandos simples, documentación interna |
| Complejidad de operaciones privilegiadas | Media    | Alto    | Diseñar el modelo de privilegios en la Fase 1 |
| Tiempo de desarrollo                | Media        | Medio   | Priorizar MVP (Home + Drivers + Recommended) |
| Diferencias sutiles de WebView entre distros | Baja   | Bajo    | CSS robusto + pruebas en varias distros |

---

## 11. Criterios de éxito de la 3.0

- [ ] Look visualmente idéntico en XFCE, Plasma, GNOME y al menos un compositor tiling
- [ ] Todas las funciones principales de la 2.x presentes o mejoradas
- [ ] Rendimiento claramente superior a la versión Python actual
- [ ] Empaquetado .deb limpio e instalable
- [ ] Sistema de privilegios seguro y usable
- [ ] Soporte de los 8 idiomas actuales
- [ ] Código documentado y estructura clara para futuros contribuidores

---

## 12. Recomendación final

**Proceder con Tauri 2 + frontend web moderno + lógica en Rust.**

Esta combinación es la que mejor satisface:
- El requisito de aspecto idéntico en cualquier entorno
- La necesidad de mejor rendimiento
- La voluntad de hacer una base limpia y a largo plazo
- El carácter de herramienta de sistema de Soplos Welcome

La alternativa más cercana (Python + WebView) sigue siendo válida si se prioriza velocidad de desarrollo a corto plazo, pero Tauri 2 ofrece una base superior para los próximos años.

---

## 13. Próximos pasos inmediatos sugeridos

1. Validar este informe y la dirección visual del boceto.
2. Crear el repositorio `soplos-welcome-3`.
3. Generar el proyecto base con `npm create tauri-app`.
4. Implementar el shell visual (sidebar + Home) según el boceto.
5. Añadir los primeros comandos Rust de información del sistema.
6. Probar en los tres escritorios principales + un tiling WM.

---

## Anexos

### A. Boceto HTML
El boceto visual completo se encuentra en el archivo  
`mockup-soplos-welcome-3.html` (generado en la conversación).

### B. Referencias técnicas
- Documentación oficial Tauri 2: https://v2.tauri.app
- Sidecar / external binaries: https://v2.tauri.app/develop/sidecar/
- Capabilities & Permissions: https://v2.tauri.app/security/
- Empaquetado Debian: https://v2.tauri.app/distribute/debian/

### C. Repositorio actual de referencia
- https://github.com/SoplosLinux/soplos-welcome

---

**Fin del informe.**

*Documento generado para uso interno del proyecto Soplos Linux.  
Puede copiarse directamente a VS Code o a un repositorio de documentación.*
