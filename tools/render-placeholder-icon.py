"""Render the placeholder application icon the `desktop` form ships.

    python tools/render-placeholder-icon.py

Writes `src/templates/desktop/icons/icon.png` and `icon.ico`.

Why this exists at all. A Tauri build on Windows fails outright without an
`.ico`, so the form has to ship one before the product has a mark of its own.
Until now that was Tauri's own `??` placeholder, which says nothing about the
line and, worse, looked deliberate enough that nobody replaced it. The line has
an umbrella mark for exactly this - lambda on graphite - and it reads as "this
is a lacodda project whose own mark has not been chosen yet".

Why the mark is drawn here rather than rasterized from the registry's SVG. The
masters live in the vault, which is private and is not a build input of a
public crate; and no SVG rasterizer is installed on the line. The geometry is
eleven numbers, so it is spelled out, and `tests/placeholder_icon.rs` holds the
result to the rule rather than to this script's word.

The level rule of the line, applied per image inside the `.ico` (not to the
file as a whole - that mistake put a flat coloured lozenge on kilna's desktop):

    S  <= 27px   the hexagon filled with the mark's colour, dark glyph
    M  28..63px  the dark plate, outlined in the colour, colour glyph
    L  >= 64px   the same as M here: the umbrella mark carries no metaphor,
                 so its L and M differ only in how large the glyph is set

Largest image first inside the container: Windows picks by nearest size and
ignores order, but `tauri-codegen` takes `entries()[0]` verbatim as the window
icon, and a titlebar stretched out of a 16px image is how kilna v0.32.1 shipped.
"""

import io
import math
import os
import struct
import sys

from PIL import Image, ImageDraw, ImageFont

# The line's palette, as `Projects/brand/README.md` fixes it.
GRAPHITE = (0x8E, 0x9A, 0xA3, 0xFF)  # the umbrella mark's colour
PLATE = (0x1B, 0x21, 0x26, 0xFF)  # the dark tile, identical in both themes
DARK_GLYPH = (0x16, 0x19, 0x1C, 0xFF)  # the S glyph on a light fill

# The hexagon of the line, in the masters' 100x100 viewBox. Pointy-top, as
# cargo's is.
HEX = [(50, 5), (89, 27.5), (89, 72.5), (50, 95), (11, 72.5), (11, 27.5)]

# The masters do not draw this polygon directly: they stroke it 9 units wide
# in the tile's own colour, with `stroke-linejoin="round"`. That grows the
# shape by half the stroke and rounds every corner - the rounded hexagon is
# the mark, and a bare polygon here has six sharp points instead. Compared
# against the master in a browser, which is how the difference was caught.
STROKE = 9
OUTLINE_STROKE = 3.5  # the colour outline laid over the plate, on M and L

# Rendering happens here and is scaled down to each size wanted: a hexagon
# edge and a glyph stem both land between pixels at 16px, and downsampling a
# large clean draw is what keeps them from breaking up.
SUPERSAMPLE = 2048

# What goes into the .ico. Chosen to cover what Windows actually asks for: 16
# and 24 for the titlebar and Alt+Tab, 32 and 48 for the desktop and Explorer,
# 64 and up for large icons and the task switcher.
ICO_SIZES = [256, 128, 64, 48, 32, 24, 16]

# Tauri's bundler reads this one for every non-Windows target.
PNG_SIZE = 512

# What a web app manifest and iOS ask for, beside the 512px image above.
PWA_ICONS = [("icon-192.png", 192), ("apple-touch-icon.png", 180)]

# The level boundary of the line: at 27px and below the outline collapses into
# noise, so the filled tile is all that survives.
S_CEILING = 27

FONTS = [
    r"C:\Windows\Fonts\CascadiaCode.ttf",
    r"C:\Windows\Fonts\CascadiaMono.ttf",
    "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Bold.ttf",
    r"C:\Windows\Fonts\consolab.ttf",
]


def font_at(pixels):
    """The heaviest monospace face available, set to `pixels`.

    The masters ask for weight 700 on the plated tile and 800 on the filled
    one. Cascadia's variable axis stops at 700, and a synthetic heavier weight
    is a smear rather than a face, so 700 is used for both: at 16px the
    difference between 700 and 800 is below one pixel of stem anyway.
    """
    for path in FONTS:
        if not os.path.exists(path):
            continue
        face = ImageFont.truetype(path, pixels)
        try:
            face.set_variation_by_name("Bold")
        except (OSError, AttributeError):
            pass  # A static face is already whatever weight it is.
        return face
    raise SystemExit("no monospace font found; install Cascadia Code or JetBrains Mono")


def scaled(points, size):
    """The 100x100 master geometry, in the pixels of an image of `size`."""
    return [(x * size / 100, y * size / 100) for x, y in points]


def grown(points, amount):
    """Every edge moved outwards along its own normal by `amount`.

    Half of what a centred stroke does; the other half is the rounding below.

    By the edge normals rather than by pushing each vertex along its radius:
    the line's hexagon is not equilateral (the vertical edges sit 39 from the
    centre, the vertices 45), so a radial push moves those edges by only
    39/45 of what it moves the corners, and the outline ends up floating
    inside the plate instead of sitting on its rim. Measured on the 512px
    render against the master: a 12px band of bare plate outside the outline.
    """
    def outward(start, end):
        """The unit normal of an edge, pointing away from the centre."""
        dx, dy = end[0] - start[0], end[1] - start[1]
        span = math.hypot(dx, dy)
        nx, ny = dy / span, -dx / span
        # Which of the two normals points outwards depends on the winding, so
        # it is checked against the centre rather than assumed.
        midx, midy = (start[0] + end[0]) / 2, (start[1] + end[1]) / 2
        return (nx, ny) if (midx - cx) * nx + (midy - cy) * ny > 0 else (-nx, -ny)

    cx = sum(x for x, _ in points) / len(points)
    cy = sum(y for _, y in points) / len(points)

    out = []
    for index in range(len(points)):
        before, corner, after = points[index - 1], points[index], points[(index + 1) % len(points)]
        # Each edge becomes the line through its offset midpoint with the same
        # direction; the new corner is where the two offset lines cross.
        first_normal = outward(before, corner)
        second_normal = outward(corner, after)
        first_point = (corner[0] + first_normal[0] * amount, corner[1] + first_normal[1] * amount)
        second_point = (corner[0] + second_normal[0] * amount, corner[1] + second_normal[1] * amount)

        first_direction = (corner[0] - before[0], corner[1] - before[1])
        second_direction = (after[0] - corner[0], after[1] - corner[1])
        cross = first_direction[0] * second_direction[1] - first_direction[1] * second_direction[0]
        if abs(cross) < 1e-9:
            out.append(first_point)  # Collinear edges: no corner to resolve.
            continue
        gap = (second_point[0] - first_point[0], second_point[1] - first_point[1])
        along = (gap[0] * second_direction[1] - gap[1] * second_direction[0]) / cross
        out.append((first_point[0] + first_direction[0] * along, first_point[1] + first_direction[1] * along))
    return out


def rounded(points, radius, steps=32):
    """The polygon with every corner replaced by an arc of `radius`.

    The other half of `stroke-linejoin="round"`. Each corner becomes the arc
    that is tangent to both edges: walk in from the vertex along each edge by
    `radius / tan(half-angle)`, put the arc's centre on the bisector at
    `radius / sin(half-angle)`, and sweep between the two tangent points.
    """
    out = []
    for index, corner in enumerate(points):
        previous = points[index - 1]
        following = points[(index + 1) % len(points)]

        def unit(origin, towards):
            dx, dy = towards[0] - origin[0], towards[1] - origin[1]
            span = math.hypot(dx, dy)
            return dx / span, dy / span

        ax, ay = unit(corner, previous)
        bx, by = unit(corner, following)
        half = math.acos(max(-1.0, min(1.0, ax * bx + ay * by))) / 2

        along = radius / math.tan(half)
        start = (corner[0] + ax * along, corner[1] + ay * along)
        end = (corner[0] + bx * along, corner[1] + by * along)

        bisector_x, bisector_y = ax + bx, ay + by
        bisector = math.hypot(bisector_x, bisector_y)
        reach = radius / math.sin(half)
        centre = (corner[0] + bisector_x / bisector * reach, corner[1] + bisector_y / bisector * reach)

        first = math.atan2(start[1] - centre[1], start[0] - centre[0])
        last = math.atan2(end[1] - centre[1], end[0] - centre[0])
        # The short way round: the long way would cut across the tile.
        while last - first > math.pi:
            last -= 2 * math.pi
        while first - last > math.pi:
            last += 2 * math.pi

        for step in range(steps + 1):
            angle = first + (last - first) * step / steps
            out.append((centre[0] + math.cos(angle) * radius, centre[1] + math.sin(angle) * radius))
    return out


def tile_shape(size, stroke):
    """The rounded hexagon a centred stroke of `stroke` units produces.

    A negative `stroke` gives the inner edge of that stroke instead of the
    outer one - the two together are the ring the outline is drawn as. The
    rounding radius stays positive: an inner corner is rounded too, by the
    same stroke that rounds the outer one.
    """
    reach = stroke / 2 * size / 100
    return rounded(grown(scaled(HEX, size), reach), abs(reach))


def draw_glyph(image, size, colour, font_size_in_master, baseline_in_master):
    """Set the lambda where the master sets it.

    The masters position text by SVG's baseline and `text-anchor="middle"`.
    Pillow anchors by the glyph's own box, so the baseline is asked for
    explicitly (`anchor="ms"`): centring on the ink instead would sit the
    lambda a little high, because its descender-less form has no counterweight
    below the baseline.
    """
    draw = ImageDraw.Draw(image)
    face = font_at(round(font_size_in_master * size / 100))
    draw.text(
        (size / 2, baseline_in_master * size / 100),
        "\u03bb",
        font=face,
        fill=colour,
        anchor="ms",
    )


def tile(size, level):
    """One image of the umbrella mark, at `level` ("S" or "M").

    "L" is not a case of its own: the umbrella mark has no metaphor to put
    under the code, so the L master and the M master differ only in the size
    of the glyph, and above 64px the M drawing is the mark.
    """
    big = Image.new("RGBA", (SUPERSAMPLE, SUPERSAMPLE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(big)
    plate = tile_shape(SUPERSAMPLE, STROKE)

    if level == "S":
        # Filled tile, dark glyph. The master judges the glyph's colour by the
        # fill's luminance; graphite sits above the line, so the glyph is dark.
        # Measured: 6.14:1 against graphite, against 2.7:1 for a light glyph.
        draw.polygon(plate, fill=GRAPHITE)
        draw_glyph(big, SUPERSAMPLE, DARK_GLYPH, 46, 66)
    else:
        # The plate with the mark's colour outlined on it. The master strokes
        # the same hexagon 3.5 wide at 75% opacity; that is drawn here as a
        # ring between two rounded contours and flattened against the plate,
        # because an `.ico` entry carries one alpha channel and a
        # half-transparent stroke over transparency leaves a ghost edge.
        outline_colour = tuple(round(GRAPHITE[channel] * 0.75 + PLATE[channel] * 0.25) for channel in range(3)) + (0xFF,)
        draw.polygon(plate, fill=PLATE)
        draw.polygon(tile_shape(SUPERSAMPLE, OUTLINE_STROKE), fill=outline_colour)
        draw.polygon(tile_shape(SUPERSAMPLE, -OUTLINE_STROKE), fill=PLATE)
        draw_glyph(big, SUPERSAMPLE, GRAPHITE, 40, 64)

    return big.resize((size, size), Image.LANCZOS)


def level_for(size):
    return "S" if size <= S_CEILING else "M"


def build_ico(images):
    """Pack `[(size, image)]` into an `.ico`, in the order given.

    Written out by hand rather than handed to Pillow's ICO writer, which sorts
    the sizes ascending no matter what order it is given them in - so the
    16px image lands first, which is the entry `tauri-codegen` takes verbatim
    for the window. Measured, not assumed: the first draft of this script used
    Pillow and produced exactly that container.

    Each payload is a PNG. The format has allowed that since Vista, every
    reader the line's apps meet handles it, and it keeps the file a third of
    the size of the BMP encoding with its separate AND mask.
    """
    header = struct.pack("<HHH", 0, 1, len(images))  # reserved, type 1 = icon, count
    directory, payloads = b"", b""
    offset = len(header) + 16 * len(images)

    for size, image in images:
        buffer = io.BytesIO()
        image.save(buffer, "PNG")
        payload = buffer.getvalue()
        directory += struct.pack(
            "<BBBBHHII",
            0 if size >= 256 else size,  # width: 0 means 256, which does not fit in a byte
            0 if size >= 256 else size,  # height, likewise
            0,  # palette colours: 0 for a truecolour image
            0,  # reserved
            1,  # colour planes
            32,  # bits per pixel
            len(payload),
            offset,
        )
        payloads += payload
        offset += len(payload)

    return header + directory + payloads


def main():
    here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons = os.path.join(here, "src", "templates", "desktop", "icons")
    os.makedirs(icons, exist_ok=True)

    png = tile(PNG_SIZE, level_for(PNG_SIZE))
    png.save(os.path.join(icons, "icon.png"), "PNG")

    with open(os.path.join(icons, "icon.ico"), "wb") as out:
        out.write(build_ico([(size, tile(size, level_for(size))) for size in ICO_SIZES]))

    print(f"wrote icon.png ({PNG_SIZE}px) and icon.ico ({', '.join(str(s) for s in ICO_SIZES)}) -> {icons}")

    # The spa form's `--with pwa`: the icons an install asks for. All of them
    # are well above the S ceiling, so all are the plated mark; the 512px one
    # is the desktop's `icon.png`, carried by the spa form as it is.
    public = os.path.join(here, "src", "templates", "spa", "public")
    os.makedirs(public, exist_ok=True)
    for name, size in PWA_ICONS:
        tile(size, level_for(size)).save(os.path.join(public, name), "PNG")
    print(f"wrote {', '.join(f'{n} ({s}px)' for n, s in PWA_ICONS)} -> {public}")


if __name__ == "__main__":
    sys.exit(main())
