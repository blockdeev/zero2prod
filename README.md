# zero2prod

Proyecto Rust siguiendo el libro *Zero To Production In Rust*.

> 📌 **Nota:** este README se irá actualizando a medida que avance el proyecto. Por ahora documenta el setup inicial del entorno y del pipeline de CI.

---

## 🛠️ Requisitos previos

Antes de tocar el código, necesitás tener instalado:

- [Rust](https://www.rust-lang.org/tools/install) (vía `rustup`)
- `cargo` (viene incluido con `rustup`)

Rust y Cargo se instalan juntos, pero **algunas herramientas que usamos en este proyecto NO vienen por defecto** y hay que instalarlas aparte.

---

## 📦 Herramientas adicionales a instalar

`rustup` solo instala el compilador (`rustc`) y el gestor de paquetes (`cargo`). Las siguientes herramientas son *subcomandos* de Cargo hechos por la comunidad, y hay que instalarlas manualmente antes de trabajar en este repo:

```bash
cargo install cargo-watch
cargo install cargo-audit
```

| Herramienta | Para qué sirve |
|---|---|
| **cargo-watch** | Recompila y corre el proyecto automáticamente cada vez que guardás un archivo. Ahorra tener que tipear `cargo run` a mano en cada cambio. |
| **cargo-audit** | Escanea las dependencias del proyecto contra una base de datos pública de vulnerabilidades conocidas ([RustSec](https://rustsec.org/)). |

Una vez instaladas, quedan disponibles como comandos de `cargo`:

```bash
cargo watch -x check
cargo audit
```

---

## ⚡ Loop de desarrollo rápido: `cargo check`

Compilar un proyecto Rust completo (`cargo build` o `cargo run`) puede ser lento, porque incluye generar el binario final y linkearlo.

Para iterar rápido mientras programás, usamos:

```bash
cargo check
```

- ✅ Verifica que el código compile (tipos, sintaxis, borrow checker) sin generar el binario final.
- ✅ Es mucho más rápido que un build completo.
- ❌ No genera un ejecutable — no sirve para correr el programa, solo para chequear errores.

Combinado con `cargo-watch`, queda un loop de desarrollo cómodo:

```bash
cargo watch -x check -x test -x run
```

Esto corre, en cada guardado: primero `check` (rápido), después `test`, y si todo pasa, `run`.

---

## 🏗️ Estructura del proyecto

Este proyecto está separado en **librería + binario**, un patrón común en proyectos Rust que exponen una API:

```
zero2prod/
├── src/
│   ├── lib.rs      ← toda la lógica real: rutas, handlers, configuración del servidor
│   └── main.rs      ← entrypoint mínimo: arma un TcpListener y arranca lo que expone lib.rs
└── tests/
    └── health_check.rs   ← tests de integración, le pegan a la API por HTTP real
```

¿Por qué separado así? Porque un binario (`main.rs`) no se puede importar como dependencia desde otro archivo. Al mover la lógica a `lib.rs`, los tests en `tests/` pueden hacer `use zero2prod::run` y levantar el servidor real para probarlo end-to-end, tal como lo haría un cliente HTTP externo.

📖 Para una explicación más profunda de los conceptos internos (async, `Future`, el runtime Tokio, extractors, el trait `Service`), ver [`marcoteorico.md`](./marcoteorico.md).

---

## 🌐 Endpoints disponibles

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/health_check` | Devuelve `200 OK` sin body. Usado para verificar que el servidor está vivo. |

---

## 🧪 Testing

El proyecto usa **tests de integración**, ubicados en `tests/`, que levantan el servidor real en un puerto aleatorio y le hacen requests HTTP de verdad (con el crate `reqwest`), simulando exactamente lo que haría un cliente externo.

```bash
cargo test
```

Puntos clave de cómo están armados:

- Cada test arranca el servidor con `tokio::spawn`, no con `.await` directo — esto es importante, porque `.await` sobre un servidor bloquearía indefinidamente (los servidores HTTP escuchan para siempre, nunca "terminan" solos).
- Se usa el puerto `0` al hacer el `bind`, lo cual le pide al sistema operativo que asigne un puerto disponible al azar. Esto evita colisiones si corrés varios tests en paralelo, o si el puerto fijo de producción (`8080`) ya está ocupado.

---

## 📦 Dependencias de testing (dev-dependencies)

Estas solo se compilan al correr `cargo test`, no forman parte del binario final:

| Crate | Para qué |
|---|---|
| `reqwest` (con `rustls`, no TLS nativo) | Cliente HTTP para simular requests reales contra la API en los tests |
| `tokio` (features `macros`, `rt-multi-thread`) | Habilita `#[tokio::test]` y `tokio::spawn` para correr el servidor en background durante los tests |

> 💡 Usamos `rustls` en vez del TLS nativo del sistema para evitar depender de OpenSSL/`pkg-config` instalados a nivel del sistema operativo — hace que el proyecto compile igual en cualquier máquina, sin pasos de instalación extra.

---

## ✅ CI Pipeline (GitHub Actions)

Este repo corre chequeos automáticos en cada `push` y `pull request`, usando GitHub Actions. Los workflows viven en `.github/workflows/`.

### `general.yml`

Corre tres jobs en paralelo:

- **Test** → corre la suite de tests (`cargo test`), incluyendo los tests de integración.
- **Rustfmt** → verifica que el código esté formateado según el estándar de Rust (`cargo fmt --check`).
- **Clippy** → corre el linter oficial de Rust, que detecta errores comunes y malas prácticas (`cargo clippy`).

### `audit.yml`

Corre `cargo-audit` directamente (instalado en el runner) para detectar vulnerabilidades conocidas en las dependencias. Se dispara:

- Cada vez que cambia `Cargo.toml` o `Cargo.lock`.
- Una vez al día, de forma programada (por si aparece una vulnerabilidad nueva en una dependencia que ya estaba en el proyecto).
- Manualmente, desde la pestaña **Actions** de GitHub (`workflow_dispatch`).

> Podés ver el resultado de cada corrida en la pestaña **Actions** del repositorio en GitHub.

---

## 🚀 Cómo levantar el proyecto localmente

```bash
git clone git@github.com:blockdeev/zero2prod.git
cd zero2prod
cargo watch -x check -x test -x run
```

El servidor queda escuchando en `http://127.0.0.1:8080`. Podés probar el health check con:

```bash
curl -v http://127.0.0.1:8080/health_check
```
