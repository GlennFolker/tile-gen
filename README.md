# `tile-gen`
A simple [Rust (🚀)](https://www.rust-lang.org/) project for generating bitmasked tile variants.

- - -

**Raw Image**: <br>
![example](/example.png) <br>
**Processed Image**: <br>
![example-tiled](/example-tiled.png) <br>

- - -

### Using the Generator
- Download the binary suitable for your OS and architecture from the [releases](https://github.com/GlennFolker/tile-gen/releases).
- Create your sprite:
  - The dimension size must be divisible by 4 and must be square.
  - Pixel scaling should be 1×1.
- Run `tile-gen proc [your-sprite].png`. This will generate a new processed sprite with the name as your raw sprite name suffixed with `-tiled` (i.e., `[your-sprite]-tiled.png`).

### Using the Processed Sprite
- Split your sprites into 47 elements as shown below: <br>
  ![tile-indexing](/example-tiled-indexing.png)
- Count the mask by using `mask |= 1 << index`; treat `index` as: <br>
  ![mask-mapping](/mask-mapping.png)
- The sprite of the tile can be mapped from the `mask` by using `sprites[tiles[mask]]` where `tiles` is an array of the mapping returned by running `tile-gen mapping`:
  ```
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
 3,  4,  3,  4, 15, 40, 15, 20,  3,  4,  3,  4, 15, 40, 15, 20,
 5, 28,  5, 28, 29, 10, 29, 23,  5, 28,  5, 28, 31, 11, 31, 32,
 3,  4,  3,  4, 15, 40, 15, 20,  3,  4,  3,  4, 15, 40, 15, 20,
 2, 30,  2, 30,  9, 46,  9, 22,  2, 30,  2, 30, 14, 44, 14,  6,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
 3,  0,  3,  0, 15, 42, 15, 12,  3,  0,  3,  0, 15, 42, 15, 12,
 5,  8,  5,  8, 29, 35, 29, 33,  5,  8,  5,  8, 31, 34, 31,  7,
 3,  0,  3,  0, 15, 42, 15, 12,  3,  0,  3,  0, 15, 42, 15, 12,
 2,  1,  2,  1,  9, 45,  9, 19,  2,  1,  2,  1, 14, 18, 14, 13,
  ```
