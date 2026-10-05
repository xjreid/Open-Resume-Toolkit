from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'logo'
OUT.mkdir(exist_ok=True)
TEAL = '#28676C'
DEEP = '#1D4F53'
# Flat geometry: two upright document leaves with an open, rising spine.
LEFT = 'M16 26C16 21.6 19.6 18 24 18H41C47 18 52 21 56 25V88C51 84 46 82 40 82H24C19.6 82 16 78.4 16 74Z'
RIGHT = 'M64 25L93 12C97.5 10 102 13 102 18V69C102 72 100.2 74.7 97.5 76L64 91Z'
def mark(color, second=None):
    return f'<path d="{LEFT}" fill="{color}"/><path d="{RIGHT}" fill="{second or color}"/>'
def svg(viewbox, content):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{viewbox}" role="img"><title>Open Resume Toolkit — Open Folio</title>{content}</svg>'
(OUT/'open-folio-mark.svg').write_text(svg('0 0 120 104',mark(TEAL)))
(OUT/'open-folio-reversed.svg').write_text(svg('0 0 120 104',mark('#FFFFFF')))
app = f'<rect x="64" y="64" width="896" height="896" rx="208" fill="{DEEP}"/><g transform="translate(152 200) scale(6)">{mark("#FFFFFF", "#C9E6E8")}</g>'
(OUT/'open-folio-app.svg').write_text(svg('0 0 1024 1024',app))
font = TTFont(ROOT/'source'/'HankenGrotesk.ttf')
from fontTools.varLib.instancer import instantiateVariableFont
font = instantiateVariableFont(font, {'wght':600})
glyphs=font.getGlyphSet(); cmap=font.getBestCmap(); units=font['head'].unitsPerEm
def textpaths(text,x,y,size):
    parts=[]; cursor=0; scale=size/units
    for char in text:
        glyph=glyphs[cmap[ord(char)]]
        pen=SVGPathPen(glyphs); glyph.draw(pen)
        parts.append(f'<path d="{pen.getCommands()}" transform="translate({x+cursor*scale:.3f} {y}) scale({scale:.7f} {-scale:.7f})"/>')
        cursor+=glyph.width
    return ''.join(parts)
lockup=f'<g transform="translate(0 9) scale(.92)">{mark(TEAL)}</g><g fill="{DEEP}">{textpaths("Open Resume",136,51,37)}{textpaths("Toolkit",136,94,37)}</g>'
(OUT/'open-folio-lockup.svg').write_text(svg('0 0 410 115',lockup))
print('Created four font-independent SVG logo masters.')
