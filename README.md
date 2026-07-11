# zero2prod

Proyecto Rust basado en el libro *Zero To Production In Rust*.

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

## ✅ CI Pipeline (GitHub Actions)

Este repo corre chequeos automáticos en cada `push` y `pull request`, usando GitHub Actions. Los workflows viven en `.github/workflows/`.

### `general.yml`

Corre tres jobs en paralelo:

- **Test** → corre la suite de tests (`cargo test`).
- **Rustfmt** → verifica que el código esté formateado según el estándar de Rust (`cargo fmt --check`).
- **Clippy** → corre el linter oficial de Rust, que detecta errores comunes y malas prácticas (`cargo clippy`).

### `audit.yml`

Corre `cargo-audit` para detectar vulnerabilidades conocidas en las dependencias. Se dispara:

- Cada vez que cambia `Cargo.toml` o `Cargo.lock`.
- Una vez al día, de forma programada (por si aparece una vulnerabilidad nueva en una dependencia que ya estaba en el proyecto).

> Podés ver el resultado de cada corrida en la pestaña **Actions** del repositorio en GitHub.

---

## 🚀 Cómo levantar el proyecto localmente

```bash
git clone git@github.com:blockdeev/zero2prod.git
cd zero2prod
cargo watch -x check -x test -x run
```
