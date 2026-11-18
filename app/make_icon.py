"""Draws the Edge Hub app icon (build-time only). Outputs icon.ico, icon64.rgba, icon.png, icon_b64.txt."""
import base64, io, os
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
S = 1024  # supersampled canvas


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(len(a)))


def draw():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    # rounded-square tile with vertical gradient
    grad = Image.new("RGBA", (S, S))
    gd = ImageDraw.Draw(grad)
    for y in range(S):
        gd.line([(0, y), (S, y)], fill=lerp((44, 12, 20, 255), (10, 10, 13, 255), y / S))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([40, 40, S - 40, S - 40], radius=220, fill=255)
    img.paste(grad, (0, 0), mask)
    # red glow
    glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse([200, 260, 824, 820], fill=(255, 42, 61, 120))
    glow = glow.filter(ImageFilter.GaussianBlur(110))
    img = Image.alpha_composite(img, Image.composite(glow, Image.new("RGBA", (S, S)), mask))
    # rim
    rim = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(rim).rounded_rectangle([40, 40, S - 40, S - 40], radius=220, outline=(255, 70, 90, 150), width=10)
    img = Image.alpha_composite(img, rim)

    # controller silhouette
    pad = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(pad)
    white = (246, 246, 250, 255)
    d.rounded_rectangle([210, 350, 814, 600], radius=120, fill=white)          # body
    d.ellipse([170, 420, 420, 790], fill=white)                                  # left grip
    d.ellipse([604, 420, 854, 790], fill=white)                                  # right grip
    d.rounded_rectangle([250, 300, 430, 380], radius=40, fill=(255, 42, 61, 255))  # L bumper
    d.rounded_rectangle([594, 300, 774, 380], radius=40, fill=(255, 42, 61, 255))  # R bumper
    d.rounded_rectangle([210, 350, 814, 600], radius=120, fill=white)          # re-cover bumper bases
    cut = (22, 12, 16, 255)
    # d-pad
    d.rounded_rectangle([288, 438, 318, 538], radius=8, fill=cut)
    d.rounded_rectangle([253, 473, 353, 503], radius=8, fill=cut)
    # face buttons
    for cx, cy in ((736, 448), (776, 488), (736, 528), (696, 488)):
        d.ellipse([cx - 19, cy - 19, cx + 19, cy + 19], fill=(255, 42, 61, 255))
    # sticks
    for cx in (412, 612):
        d.ellipse([cx - 52, 540, cx + 52, 644], fill=cut)
        d.ellipse([cx - 30, 562, cx + 30, 622], fill=(70, 70, 82, 255))
    # light bar
    d.rounded_rectangle([452, 372, 572, 392], radius=10, fill=(255, 42, 61, 255))
    shadow = pad.split()[3].point(lambda a: int(a * 0.55))
    sh = Image.new("RGBA", (S, S), (0, 0, 0, 255))
    sh.putalpha(shadow)
    sh = sh.filter(ImageFilter.GaussianBlur(18))
    img.alpha_composite(sh, (0, 22))
    img.alpha_composite(pad)
    return img


def main():
    big = draw()
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    frames = [big.resize((s, s), Image.LANCZOS) for s in sizes]
    frames[-1].save(os.path.join(HERE, "icon.ico"), sizes=[(s, s) for s in sizes], append_images=frames[:-1])
    frames[sizes.index(256)].save(os.path.join(HERE, "icon.png"))
    with open(os.path.join(HERE, "icon64.rgba"), "wb") as f:
        f.write(big.resize((64, 64), Image.LANCZOS).tobytes())
    b = io.BytesIO()
    big.resize((96, 96), Image.LANCZOS).save(b, "PNG")
    with open(os.path.join(HERE, "icon_b64.txt"), "w") as f:
        f.write(base64.b64encode(b.getvalue()).decode())
    print("icon written")


if __name__ == "__main__":
    main()
