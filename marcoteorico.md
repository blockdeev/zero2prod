# 📚 Marco Teórico — Async, Futures, Tokio y Actix Web

> 🎯 **Objetivo de este documento:** explicar qué pasa "detrás de escena" cuando corremos nuestra API en Rust con Actix Web, conectando la teoría del libro *Zero To Production In Rust* (que usa sintaxis de 2022) con el código moderno del proyecto (macros de atributo, extractors, etc.).
>
> 📌 Complementa a `README.md` — ahí está el setup del proyecto, acá está la teoría detrás del código.

---

## 🗺️ Mapa general: qué pasa cuando llega un request

```
   Request HTTP llega
          │
          ▼
   ┌─────────────────┐
   │   HttpServer     │  ← maneja TCP / TLS / conexiones concurrentes
   └────────┬─────────┘
            ▼
   ┌─────────────────┐
   │       App        │  ← busca qué handler matchea la ruta
   └────────┬─────────┘
            ▼
   ┌─────────────────┐
   │   Extractors      │  ← resuelve los parámetros del handler (Path, Json, etc.)
   └────────┬─────────┘
            ▼
   ┌─────────────────┐
   │  Handler (async)  │  ← se ejecuta como una Future
   └────────┬─────────┘
            ▼
   ┌─────────────────┐
   │  Tokio (runtime)  │  ← hace "poll" de la Future hasta resolverla
   └────────┬─────────┘
            ▼
   ┌─────────────────┐
   │    Responder      │  ← convierte el resultado en HttpResponse
   └────────┬─────────┘
            ▼
      Response al cliente
```

---

## 1️⃣ `HttpServer` — el motor de transporte

`HttpServer` es la pieza que se encarga de **todo lo relacionado a la conexión física**, no de la lógica de negocio:

| Se encarga de... | Ejemplo |
|---|---|
| 🔌 Dónde escuchar | IP + puerto (`127.0.0.1:8080`), o un socket Unix |
| 🚦 Concurrencia | Cuántas conexiones simultáneas se permiten |
| 🔒 Seguridad de transporte | Habilitar o no TLS |

```rust
HttpServer::new(|| App::new().service(index).service(hello))
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
```

> 💡 Esto **no cambió** entre la versión del libro y la versión moderna — sigue siendo exactamente el mismo rol.

---

## 2️⃣ `App` — donde vive la lógica

`App` es un **builder pattern**: se arma encadenando métodos (`.service()`, `.route()`, `.wrap()` para middlewares, etc.), empezando de una base vacía con `App::new()`.

Su trabajo: **tomar un request y devolver una response**, decidiendo internamente a qué handler mandarlo.

---

## 3️⃣ Routing: estilo libro vs. estilo moderno

### 📖 Como lo enseña el libro (routing manual)

```rust
App::new()
    .route("/", web::get().to(greet))
    .route("/{name}", web::get().to(greet))
```

### ⚡ Como lo escribimos hoy (macros de atributo)

```rust
#[get("/")]
async fn index() -> impl Responder {
    "Hello, World!"
}

App::new().service(index)
```

> ✅ **Son equivalentes.** `#[get("/")]` es una *macro procedural* que, en tiempo de compilación, genera automáticamente el mismo patrón `Route` + `Guard` que el libro escribe a mano.

### 🔬 Prueba real (con `cargo expand`)

Expandiendo el macro, se confirma que genera exactamente esto por detrás:

```rust
let __resource = ::actix_web::Resource::new("/")
    .name("index")
    .guard(::actix_web::guard::Get())
    .to(index);
```

Es decir: la macro **no reemplaza el concepto de `Guard`** que enseña el libro (una condición que el request debe cumplir para matchear, como "ser método GET") — solo lo escribe automáticamente por nosotros.

---

## 4️⃣ Handlers y Extractors 🧩

Un **handler** es la función que procesa un request específico. El libro empieza con una única firma posible:

```rust
async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}
```

Actix permite firmas mucho más flexibles gracias a los **extractors**:

```rust
async fn hello(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", &name)
}
```

### 🧠 ¿Qué es un extractor?

Un tipo que implementa el trait `FromRequest`. Cuando un handler pide un extractor como parámetro, Actix automáticamente:

1. 🔍 Mira la parte relevante del request (URL, body, query string...)
2. ✂️ Extrae el dato correspondiente
3. 🔄 Intenta convertirlo al tipo pedido
4. ⚠️ Si falla, devuelve un `400 Bad Request` solo, sin que escribamos ese manejo de error

| Extractor | Extrae de... |
|---|---|
| `web::Path<T>` | Segmentos dinámicos de la URL (`/{name}`) |
| `web::Json<T>` | Body en formato JSON |
| `web::Query<T>` | Query string (`?page=2`) |
| `web::Data<T>` | Estado compartido de la aplicación |

---

## 5️⃣ ¿Por qué `main` no puede ser `async` directamente? ⚙️

Esto es el corazón conceptual del capítulo. Tres datos clave:

> 🔑 **Dato 1:** Una `Future` en Rust es un valor que **puede no estar listo todavía**. Necesita que algo la "polee" (`.poll()`) repetidamente para avanzar y resolverse.

> 🔑 **Dato 2:** Las futures son **lazy** — si nadie las poll-ea, nunca se ejecutan. Es un modelo *pull*, no *push*.

> 🔑 **Dato 3:** La librería estándar de Rust **no trae un runtime asíncrono incluido**, a propósito. Hay que traer uno como dependencia (Tokio, en nuestro caso).

Como no existe un runtime "oficial" reconocido por el compilador, Rust no sabe quién va a poll-ear un `main` async — por eso **no está permitido**:

```
error: `main` function is not allowed to be `async`
```

### La solución: una macro que arma todo por nosotros

| Versión | Macro |
|---|---|
| 📖 Libro | `#[tokio::main]` |
| ⚡ Proyecto actual | `#[actix_web::main]` |

Ambas expanden a algo equivalente a:

```rust
fn main() -> std::io::Result<()> {
    // arranca el runtime y bloquea el hilo principal
    // hasta que la Future se resuelva
    runtime.block_on(async {
        // tu código async original
    })
}
```

### 🔬 Prueba real, con nuestro propio `cargo expand`

```rust
fn main() -> std::io::Result<()> {
    <::actix_web::rt::System>::new()
        .block_on(async move {
            HttpServer::new(|| App::new().service(index).service(hello))
                .bind(("127.0.0.1", 8080))?
                .run()
                .await
        })
}
```

> 🎉 **Confirmado:** `#[actix_web::main]` no reemplaza a Tokio — lo esconde. `actix_web::rt::System` es un wrapper propio de Actix que **usa Tokio por debajo**. Tokio nunca se fue, solo quedó invisible detrás de la macro.

---

## 6️⃣ Cada `async fn` es una Future 🔮

```rust
#[get("/")]
async fn index() -> impl Responder {
    "Hello, World!"
}
```

Cuando el compilador ve `async fn`, **no genera una función normal**. Genera un **tipo anónimo que implementa `Future`**, con:
- El estado necesario para ejecutar el cuerpo paso a paso
- Un método `.poll()` que avanza ese estado

📌 **Cada handler genera su propia Future, independiente de las demás**, cada vez que se invoca:

- `index()` → nueva Future cada vez que llega un `GET /`
- `hello(name)` → nueva Future cada vez que llega un `GET /{name}`

Esto es lo que le permite a Tokio manejar **miles de conexiones concurrentes con pocos threads reales**: mientras una Future está "esperando" algo (ej. una consulta a base de datos), le cede el control a otra en vez de bloquear el thread.

---

## 7️⃣ ⚠️ Corrección importante: encadenar `.service()` NO ejecuta nada

```rust
App::new().service(index).service(hello)
```

Esto **no llama** a `index` ni a `hello`. Es **configuración**, no ejecución — arma una tabla interna de rutas: *"si llega algo a `/`, usar `index`; si llega algo a `/{name}`, usar `hello`"*.

### 🔄 El ciclo real

1. `HttpServer.run().await` arranca y **se queda escuchando indefinidamente**
2. Llega un request → Actix busca en la tabla cuál ruta matchea
3. **Recién ahí** se invoca el handler correspondiente, generando su Future
4. Se resuelve, se responde, y el servidor vuelve a esperar

> ✅ Un handler se puede llamar **0, 1, o miles de veces** — no una sola vez "porque está encadenado". El encadenamiento es tiempo de configuración (una sola vez, al armar la app); la ejecución del handler es tiempo de request (una y otra vez, mientras el server esté vivo).

---

## 8️⃣ ¿Qué es un `Service`? 🧱

Concepto central en el diseño de Actix — mucho más que "el método `.service()`".

```rust
trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request) -> Self::Future;
}
```

En criollo: **"algo que recibe un request y devuelve, de forma asíncrona, una response (o un error)"**.

### 🧩 Por qué es poderoso

Como *todo* implementa este mismo trait (un handler, un middleware, toda la `App`), se pueden **envolver unos dentro de otros**. Así funcionan los middlewares (logging, auth, CORS): son `Service`s que envuelven a otro `Service` interno.

```
Middleware A
   └── Middleware B
         └── Handler real (Service final)
```

### 🏭 Qué hace `.service(index)` puntualmente

Registra una **factory** (`HttpServiceFactory`) — no crea el `Service` en el momento, le da a `App` la receta para construirlo cuando haga falta (por ejemplo, cada worker thread arma su propia copia).

### ⚠️ Aclaración importante: `Service` es tiempo de compilación, Tokio opera en tiempo de ejecución

Es fácil confundir estas dos capas, así que vale la pena separarlas con claridad:

> 🔑 **`Service` es solo una interfaz** (un trait) que el compilador usa para verificar tipos. Que un handler, un middleware o `App` "implementen `Service`" significa que el compilador puede chequear, en **tiempo de compilación**, que todos comparten la misma forma: *"recibo un request, devuelvo (async) una response"*. Es pura organización de código — el compilador verifica que todo encaje, y arma las estructuras necesarias.

> 🔑 **Tokio no "maneja" `Service`s — maneja `Future`s.** Tokio es agnóstico a esta abstracción: es una capa de Actix, no de Tokio. Lo único que le importa a Tokio, un nivel más abajo, son las **futures**.

Fijate bien en la firma del trait:

```rust
trait Service<Request> {
    type Future: Future<Output = Result<Self::Response, Self::Error>>;
    fn call(&self, req: Request) -> Self::Future;
}
```

El método `.call()` de un `Service` **devuelve una `Future`**. La cadena real de eventos es:

1. 🛠️ **Tiempo de compilación** → el compilador verifica que tu handler/middleware/`App` cumplan con el trait `Service` (tipos y firmas correctas).
2. 🌐 **Tiempo de ejecución** → cuando llega un request real, Actix llama a `.call(req)` sobre el `Service` correspondiente, lo cual **produce una Future**.
3. ⚙️ **Ahí entra Tokio** → el runtime toma esa Future y la poll-ea hasta que se resuelve.

| Capa | Qué representa | Quién la usa |
|---|---|---|
| `Service` (trait) | Contrato/estructura, un contrato de tipos | El **compilador**, en tiempo de compilación |
| `Future` | Lo que efectivamente se ejecuta y se resuelve | **Tokio**, en tiempo de ejecución (polling, scheduling) |

> ✅ Tokio no "sabe" ni le importa que algo implemente `Service` — solo le importan las `Future`s que ese `Service` termina produciendo cuando se lo invoca.

---

## 9️⃣ Librería + binario: por qué separamos el proyecto en dos crates 📦

Hasta ahora todo el código vivía en `src/main.rs`, compilado como **binario**. Esto tiene un límite importante cuando queremos escribir tests de integración.

### El problema

Los tests en la carpeta `tests/` se compilan como **binarios separados**, con exactamente el mismo nivel de acceso que tendría alguien que importa nuestro proyecto como dependencia (`use zero2prod::algo`). Pero **un binario no se puede importar como dependencia** — solo las librerías (`lib`) se pueden importar con `use`. Si toda la lógica vive únicamente en `main.rs`, los tests en `tests/` no tienen a qué apuntar: no existe ningún `zero2prod::algo` al que hacer `use`, porque no hay ninguna librería, solo un ejecutable aislado.

### La solución: dos crates en el mismo `Cargo.toml`

```toml
[lib]
path = "src/lib.rs"

[[bin]]
path = "src/main.rs"
name = "zero2prod"
```

```
src/
├── lib.rs      ← TODA la lógica real vive acá (App, HttpServer, handlers, routing)
└── main.rs      ← "entrypoint fino": solo arranca lo que expone lib.rs
```

### Cómo queda `main.rs`

```rust
use std::net::TcpListener;
use zero2prod::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind random port");
    run(listener)?.await
}
```

Literalmente eso — arma un listener, se lo pasa a `run()` (que vive en la librería), y lo ejecuta.

### Por qué vale la pena, en concreto

| Beneficio | Explicación |
|---|---|
| 🧪 Habilita testing real | Sin esto, no hay ningún punto de entrada al que los tests en `tests/` puedan engancharse |
| 🛡️ `main.rs` casi imposible de romper | Cuanta menos lógica tenga el binario, menos cosas pueden fallar ahí — toda la complejidad vive en un lugar testeable |
| ♻️ Reutilización | Si mañana se necesitara un segundo binario (por ejemplo, un CLI de administración), ambos podrían importar la misma librería sin duplicar código |

---

## 🔟 Concurrencia real: dentro de UNA task vs. entre VARIAS tasks 🐢🐇

Este es un punto que suele confundir, así que vale la pena remarcarlo con cuidado — tiene que ver con una idea errónea común: *"Rust maneja la concurrencia automáticamente, así que un `.await` nunca debería bloquear nada más"*. Eso **no es del todo cierto**.

### Lo que Rust SÍ hace bien

Cuando una `Future` está esperando algo, el runtime no bloquea el *thread del sistema operativo* completo — puede cederle el procesador a **otras tasks** que también estén registradas con él.

### El matiz importante: "otras tasks" no aparecen solas

El runtime solo puede intercalar trabajo entre tasks que **ya existen**. Si escribimos código con varios `.await` en secuencia, dentro de la misma función, eso es **una sola task**, y dentro de ella no hay concurrencia:

```rust
async fn spawn_app() -> std::io::Result<()> {
    zero2prod::run().await   // (A) bloquea el avance de ESTA task hasta resolverse
}

async fn health_check_works() {
    spawn_app().await;       // (B) nunca se llega acá si (A) no termina
    let client = /* ... */;  // (C) tampoco
}
```

`.await` significa **"no avances más allá de este punto hasta que esta future se resuelva"** — dentro de una misma cadena secuencial, eso es indistinguible de código síncrono normal. El hecho de que el runtime *podría* aprovechar el thread libre para otra task no ayuda en nada acá, porque no existe ninguna otra task corriendo en paralelo.

### Cómo se crea concurrencia real: `tokio::spawn`

```rust
fn spawn_app() {
    let server = zero2prod::run().expect("Failed to bind address");
    let _ = tokio::spawn(server);   // ← crea una NUEVA task, independiente
}
```

`tokio::spawn(server)` le entrega esa future al runtime como una **task aparte**, sin esperar su resultado (a diferencia de `.await`, que sí espera). Por eso `spawn_app()` retorna casi al instante — nunca esperó a que el servidor "termine" (y un servidor HTTP, por diseño, nunca termina solo).

### 🐢🐇 Metáfora para recordarlo

- **Una task con varios `.await` en secuencia** = una persona haciendo una fila de trámites, uno detrás del otro. Si el trámite 1 nunca termina, nunca llega al trámite 2.
- **Varias tasks (`tokio::spawn`)** = varias personas en filas distintas, todas atendidas por el mismo edificio (el runtime), que va salteando entre ventanillas cuando alguna persona queda esperando algo.

> ✅ **Regla general:** dentro de una misma cadena de `.await` secuenciales, cada `.await` bloquea el avance de *esa* task. La concurrencia real entre piezas de trabajo distintas solo aparece cuando explícitamente creamos tasks separadas (`tokio::spawn`) o usamos combinadores como `join!`/`select!`.

---

## 1️⃣1️⃣ Puerto `0`: dejar que el sistema operativo elija 🎲

### El problema de usar un puerto fijo en los tests

```rust
TcpListener::bind("127.0.0.1:8080")
```

Si el puerto está *hardcodeado*, dos problemas concretos aparecen:

- Si el puerto ya está en uso (por ejemplo, por nuestra propia app corriendo con `cargo run` en otra terminal), el test falla.
- Si corremos varios tests en paralelo (comportamiento por defecto de `cargo test`), todos menos uno van a fallar tratando de tomar el mismo puerto.

### La solución: puerto `0`

```rust
let listener = TcpListener::bind("127.0.0.1:0")
    .expect("Failed to bind random port");
let port = listener.local_addr().unwrap().port();
```

El puerto `0` está **especialmente tratado a nivel del sistema operativo**: pedirle al SO que "bindee" el puerto `0` dispara una búsqueda automática de un puerto disponible, que luego se asigna a la aplicación. `listener.local_addr().unwrap().port()` nos devuelve **cuál fue el puerto real** que el SO asignó, para poder construir la URL completa y usarla en el test:

```rust
format!("http://127.0.0.1:{port}")
```

> 💡 Este patrón resuelve el problema de raíz, sin necesidad de coordinar manualmente qué puerto usa cada test — cada corrida de `cargo test` obtiene puertos frescos, sin colisiones.

### ⚠️ ¿Por qué entonces `main.rs` usa un puerto fijo (`8080`) y no `0`?

Vale la pena remarcar esta diferencia, porque a primera vista podría parecer inconsistente:

```rust
// main.rs — puerto FIJO
TcpListener::bind("127.0.0.1:8080")

// tests/health_check.rs — puerto ALEATORIO
TcpListener::bind("127.0.0.1:0")
```

Son dos escenarios con necesidades opuestas:

| | `main.rs` (producción) | `tests/` (testing) |
|---|---|---|
| **¿Quién necesita conocer el puerto?** | Clientes externos reales (navegador, `curl`, otro servicio, un balanceador de carga) | Solo el propio test, que ya lo descubre en runtime con `local_addr()` |
| **¿Se ejecuta una sola vez o muchas en paralelo?** | Una sola instancia corriendo de forma continua | Potencialmente decenas de tests corriendo en paralelo en la misma corrida de `cargo test` |
| **¿Importa que el puerto sea predecible?** | Sí — un cliente necesita saber de antemano a qué puerto conectarse (o depender de configuración/DNS que apunte a un puerto conocido) | No — cada test arma su propia URL dinámicamente, nadie externo necesita adivinarla |

En producción, un puerto aleatorio sería un problema: ¿cómo le decís al mundo exterior a qué puerto conectarse si cambia cada vez que reiniciás el servidor? Por eso se usa un puerto **fijo y conocido**, típicamente documentado o configurable (más adelante en el libro esto se vuelve configurable vía archivo de configuración, en vez de estar hardcodeado).

En testing, en cambio, el puerto es un **detalle interno y descartable**: el propio test lo pide, lo descubre (`local_addr().unwrap().port()`), arma la URL, y la usa — nadie más necesita saber cuál fue. Como además `cargo test` puede correr múltiples tests en paralelo por defecto, usar un puerto fijo ahí garantizaría colisiones; el puerto aleatorio es lo que hace posible esa paralelización sin conflictos.

> ✅ **En una frase:** el puerto fijo sirve para que el mundo exterior pueda encontrar al servidor; el puerto aleatorio sirve para que los tests no se estorben entre sí.

---

## 1️⃣2️⃣ `rustls` vs. TLS nativo: evitar depender de OpenSSL del sistema 🔒

Al agregar `reqwest` como dependencia de test (para hacer requests HTTP reales contra nuestra API), aparece una decisión de diseño relevante: **qué implementación de TLS usar**.

### El problema con la opción por defecto

Por defecto, `reqwest` usa el **TLS nativo del sistema operativo** — en Linux, eso significa compilar/enlazar contra **OpenSSL**, lo cual requiere tener `pkg-config` y las librerías de desarrollo de OpenSSL (`libssl-dev`) instaladas en la máquina. Si faltan, la compilación falla con errores como:

```
Could not find directory of OpenSSL installation...
```

### La alternativa: `rustls`

```bash
cargo add reqwest --dev --no-default-features --features rustls
```

`rustls` es una implementación de TLS **escrita íntegramente en Rust**, sin dependencias del sistema operativo. Se compila igual en cualquier máquina — la nuestra, la de un compañero de equipo, o el runner de GitHub Actions — sin pasos de instalación adicionales.

| | TLS nativo (OpenSSL) | `rustls` |
|---|---|---|
| Dependencias del sistema | Sí (`pkg-config`, `libssl-dev`) | No |
| Portabilidad | Puede fallar según el SO/distro | Compila igual en cualquier lado |
| Origen del código | C (OpenSSL) | Rust puro |

> ✅ Preferimos `rustls` por portabilidad: elimina una categoría entera de errores de compilación relacionados al entorno, tanto en desarrollo local como en CI.

---

## ✅ Resumen ejecutivo

| Concepto | Rol |
|---|---|
| **`HttpServer`** | Maneja la capa de transporte (TCP, TLS, conexiones) |
| **`App`** | Builder donde se registra routing, middlewares y handlers |
| **`#[get("/...")]`** | Macro que genera automáticamente `Route` + `Guard`, igual que el `.route()` manual del libro |
| **Extractor (`FromRequest`)** | Resuelve automáticamente los parámetros de un handler (`Path`, `Json`, `Query`, `Data`) |
| **`Future`** | Valor perezoso que necesita ser "poll-eado" para resolverse; cada `async fn` genera una |
| **Tokio (vía `actix_web::rt`)** | El runtime que arranca la macro `#[actix_web::main]` y hace el polling de las futures |
| **`Service`** | Abstracción común a handlers, middlewares y `App`: "recibe un request, devuelve una response async" |
| **Librería + binario** | Separación necesaria para que los tests de integración (`tests/`) puedan importar y ejecutar la lógica real de la app |
| **`.await` secuencial vs. `tokio::spawn`** | Un `.await` bloquea el avance de la task actual; `tokio::spawn` crea una task nueva e independiente, permitiendo concurrencia real |
| **Puerto `0`** | Le pide al sistema operativo que asigne un puerto disponible al azar, evitando colisiones en tests |
| **`rustls`** | Implementación de TLS en Rust puro, sin depender de OpenSSL/`pkg-config` del sistema |

---

## 📖 Fuente

Basado en el estudio de *Zero To Production In Rust* (Luca Palmieri), capítulo 3, secciones 3.3.2.1 a 3.5.1.2, contrastado con `cargo expand` sobre el código actual del proyecto usando `actix-web` 4.x, y con la implementación práctica del primer test de integración (`tests/health_check.rs`) usando `reqwest` + `tokio` como dev-dependencies.
