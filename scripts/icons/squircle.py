"""Put the transparent logo on a dark olive squircle for the macOS app icon.

Matches the Lockshot, Recon and Shipyard icons: an 824px superellipse inset
100px on a 1024px canvas (Apple's icon grid), a soft top-to-bottom olive
gradient, a faint drop shadow, and the logo centred inside.

Usage: squircle.py app-icon.png app-icon-macos.png
"""
import sys
import numpy as np
from PIL import Image, ImageFilter

src, dst = sys.argv[1], sys.argv[2]
SIZE, INSET = 1024, 100
EXPONENT = 4.6  # corner shape measured from the sibling icons
LOGO_WIDTH = 700  # the logo's width inside the 824px tile
SS = 4  # supersampling for a smooth edge

# Superellipse mask, drawn large and scaled down for anti-aliasing.
n = SIZE * SS
half = (SIZE - 2 * INSET) * SS / 2
c = (n - 1) / 2
y, x = np.mgrid[0:n, 0:n]
inside = (np.abs((x - c) / half) ** EXPONENT + np.abs((y - c) / half) ** EXPONENT) <= 1
mask = Image.fromarray((inside * 255).astype(np.uint8)).resize((SIZE, SIZE), Image.LANCZOS)

# Olive gradient, lighter at the top, with a gentle darkening toward the edges.
yy, xx = np.mgrid[0:SIZE, 0:SIZE] / (SIZE - 1)
top, bottom = np.array([39, 44, 31]), np.array([26, 30, 21])
grad = top[None, None, :] * (1 - yy[..., None]) + bottom[None, None, :] * yy[..., None]
vignette = 1 - 0.18 * np.clip(np.hypot(xx - 0.5, yy - 0.45) / 0.6, 0, 1) ** 2
tile_rgb = np.clip(grad * vignette[..., None], 0, 255).astype(np.uint8)
tile = Image.fromarray(tile_rgb).convert("RGBA")
tile.putalpha(mask)

# Faint shadow under the tile, like the other icons.
shadow_alpha = mask.filter(ImageFilter.GaussianBlur(10)).point(lambda v: int(v * 0.22))
canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
canvas.paste(Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 255)), (0, 6), shadow_alpha)
canvas = Image.alpha_composite(canvas, tile)

# Logo: crop to its art, scale to LOGO_WIDTH, centre on the tile.
logo = Image.open(src).convert("RGBA")
logo = logo.crop(logo.getbbox())
scale = LOGO_WIDTH / logo.width
logo = logo.resize((round(logo.width * scale), round(logo.height * scale)), Image.LANCZOS)
pos = ((SIZE - logo.width) // 2, (SIZE - logo.height) // 2 + 4)
layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
layer.paste(logo, pos, logo)
canvas = Image.alpha_composite(canvas, layer)

canvas.save(dst)
print(f"logo {logo.width}x{logo.height} at {pos} -> {dst}")
