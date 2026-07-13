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

---

## 📖 Fuente

Basado en el estudio de *Zero To Production In Rust* (Luca Palmieri), capítulo 3, secciones 3.3.2.1 a 3.3.2.4, contrastado con `cargo expand` sobre el código actual del proyecto usando `actix-web` 4.x.
