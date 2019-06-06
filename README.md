# Multi-channel signed distance fields font - tech demo

<p align="center">
  <img src="demo.png" />
</p>

Multi-channel signed distance fields is a technique, which rasterizes glyphs with distance information to low-resolution texture, which is then used to render high-quality text. You can find more information about it [here](https://steamcdn-a.akamaihd.net/apps/valve/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf) and [here](https://dspace.cvut.cz/bitstream/handle/10467/62770/F8-DP-2015-Chlumsky-Viktor-thesis.pdf).

## Goals

- Create a Rust tech demo to showcase a technique.
- Don't allocate memory during the rasterization phase.
- Glyph rasterization is parallelized to multiple CPU cores.
- All glyphs are rasterized only when needed.
- Actual font rendering and glyph rasterization don't block each other. Synchronization point is a texture copy from CPU to GPU.
- Rasterization code is separated to `sdf` library.

## UI

### Outline

- **red**, **green**, **blue** - change colour of a text.
- **inner distance** - distance from which we drawing font outline. By default, borders are in 0.5 distance. The inner side is smaller then that and outside is larger.
- **outer distance** - distance to which we draw font outline.
- **sharpness** - additional parameter, which smooths borders. High sharpness can lead to artifacts when texture's **shadow size** has a small value.

### Shadow

- **red**, **green**, **blue** - change colour of a shadow.
- **opacity** - shadow's opacity.
- **position** - distance of a center of a shadow.
- **size** - shadow's size in distance units.

### Texture

- **size** - can be made larger to fit more glyphs into one texture. It is useful because fonts can be rendered in fewer passes. Currently, the whole texture is copied due to API limitations, which will delay the rendering process.
- **font size** - the size of a rasterized glyph excluding a shadow. In principle, a higher value means higher quality, but rasterization time can grow substantially.
- **shadow size** - how many pixels there are between 0.0 to 1.0 distance. Higher values increase the quality of a shadow but decrease the quality of an edge and vice-versa.

### Render stats

You can see how the above settings impact performance.

### Other

- **show animation** - text under mouse cursor has more emphasized distance.

- **texture visibility** - when set to 1.0 shows underlying texture instead of a rendered text.
