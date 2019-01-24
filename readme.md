svg2gerber
==========

This is a simple script that reads all path's in an SVG and converts them into
a Gerber-file (also known as RS-274X; this is used to manufacture printed
circuit boards).

1. [usvg](https://docs.rs/usvg/) is used to read and simplify the SVG.

2. All paths are [flattened](https://docs.rs/lyon_algorithms/0.11.2/lyon_algorithms/geom/index.html#flattening)
   using [lyon](https://docs.rs/lyon).

3. The resulting polygons are sorted from outside in and exported to Gerber by
   translating them into alternating dark and clear shapes.


Limitations
-----------

This script was hacked together for another project of mine where I needed to
draw some layers in Inkscape. It therefore does only the absolute minimum I
required it to. Making it a bit more general shouldn't be too difficult though,
however it is unlikely I will do this. So, keep the following things in mind:

- Only filled paths are supported. No lines or outlines or anything. KiCad can
  already do that via DXF. But would be straightforward to implement.

- Paths can't intersect each other or themselves. If they do this program will
  most likely crash. This could be fixed by pre-processing all paths by
  tesselating them in lyon and tracing the outlines.

- All paths are must be closed. svg2gerber probably crashes if unclosed paths are fed to it.

- No support for Inkscape layers yet. This would require either a second 

- Gerber files actually contain annotations that describe what kind of layer a file contains.
  Currently, only basic support for different types like silkscreen etc. are implemented.


Usage
-----

    ./svg2gerb input.svg [output.gerb [layer_type]]

If no output file is specified it will take the input filename and replace the extension with "`.gerb`".

If the output path is just "`-`" the Gerber data will be printed to stdout.

`layer_type` specifies what kind of metadata the output contains. This is
optional but recommended to make it more clear for the manufacturer how to
interpret your files. Possible values are (case insensitive):

 - `F.Cu`, `B.Cu` Copper layer, positive file polarity
 - `F.Mask`, `B.Mask` Solder mask, negative file polarity (i.e. the shapes specify areas that should _not_ be covered in solder mask)
