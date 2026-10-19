# lab4-graficas

Laboratorio 4: carga el modelo OBJ de la nave creada en el laboratorio 3 y la
dibuja con un rasterizador de triángulos propio, escrito en Rust
(`minifb` para la ventana y `nalgebra-glm` como equivalente de GLM).

## Resultado

![Nave renderizada con el rasterizador propio](screenshots/nave_renderizada.png)

Vista superior ortográfica (plano X-Z) de las **1464 caras** de `assets/nave.obj`
(734 vértices), rellenas en amarillo sobre fondo oscuro. La proa (−Z) apunta
hacia arriba.

## Cómo funciona

1. **Carga del OBJ, una sola vez** (`src/obj.rs`): lee el archivo línea por
   línea. Las líneas `v` se guardan en un `Vec<Vec3>` y las `f` en `[usize; 3]`.
   A cada índice se le resta 1 (OBJ empieza en 1) y se valida que esté dentro
   del vector; las demás líneas (`#`, `o`, `s`) se ignoran.
2. **Centrado y escala** (`project_vertices()` en `src/main.rs`): calcula los
   límites de todos los vértices, toma el centro y la escala
   `min(0.8 * ancho_pantalla / ancho, 0.8 * alto_pantalla / largo)` y proyecta
   X y Z a píxeles. Se comprueba que el ancho y el largo sean mayores que cero.
   Esta transformación es estática, así que se hace antes del bucle principal.
3. **Recorrido manual de caras** (`render()` en `src/main.rs`): para cada cara
   se toman sus tres vértices y se llama a `triangle()`.
4. **Rasterizador** (`src/triangle.rs`): rellena un triángulo recorriendo su
   caja envolvente con funciones de arista y la regla top-left. Los triángulos
   degenerados (área cero en pantalla) se descartan sin dividir entre cero.
5. **Framebuffer y color** (`src/framebuffer.rs`, `src/color.rs`): `clear()`,
   `set_current_color()` y `point()`, basados en los laboratorios anteriores.
   `src/png.rs` es el escritor PNG propio del laboratorio 1.

En cada cuadro: eventos de la ventana → `clear()` → color amarillo →
`render()` → `update_with_buffer()` para presentar el framebuffer.

## Compilar y ejecutar

Requisitos: Rust (cargo) y, en Linux, las bibliotecas de X11 que usa `minifb`
(en Wayland funciona vía XWayland).

Ejecutar **desde la raíz del repositorio**, porque el modelo se carga desde la
ruta relativa `assets/nave.obj` (si no se encuentra ahí, se busca relativo a la
carpeta del proyecto):

```bash
cargo run --release
```

- `Esc` o cerrar la ventana: salir.
- `S`: guardar la captura en `screenshots/nave_renderizada.png`.

Otras opciones:

```bash
cargo run --release -- ruta/a/otro.obj            # cargar otro OBJ de triángulos
cargo run --release -- --screenshot salida.png    # dibujar un cuadro, guardar PNG y salir
```

## Estructura

```text
lab4-graficas/
├── Cargo.toml
├── assets/nave.obj            # modelo del laboratorio 3 (sin materiales)
├── screenshots/nave_renderizada.png
└── src/
    ├── main.rs                # ventana, bucle, proyección y render()
    ├── obj.rs                 # lector OBJ (v / f)
    ├── triangle.rs            # relleno de un triángulo
    ├── framebuffer.rs
    ├── color.rs
    └── png.rs                 # escritor PNG sin dependencias
```
