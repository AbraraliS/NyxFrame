import os

# Definitions
SVG_WIDTH = 512
SVG_HEIGHT = 512
STROKE_WIDTH = 32
BG_COLOR = "#070B12"
CYAN = "#00E5FF"
PURPLE = "#7C4DFF"

def generate_svg(with_bg=False):
    bg_rect = f'<rect width="{SVG_WIDTH}" height="{SVG_HEIGHT}" fill="{BG_COLOR}"/>' if with_bg else ''
    
    svg = f'''<svg width="{SVG_WIDTH}" height="{SVG_HEIGHT}" viewBox="0 0 {SVG_WIDTH} {SVG_HEIGHT}" xmlns="http://www.w3.org/2000/svg">
{bg_rect}
<defs>
    <linearGradient id="bridgeGrad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="{CYAN}" />
        <stop offset="100%" stop-color="{PURPLE}" />
    </linearGradient>
</defs>
<g stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-width="{STROKE_WIDTH}">
    <!-- Monitor Outer Frame (Cyan) -->
    <path d="M 100,380 L 100,140 C 100,120 110,110 130,110 L 300,110 C 320,110 330,120 330,140 L 330,180" stroke="{CYAN}"/>
    <!-- Monitor inner bar (F middle) -->
    <path d="M 100,245 L 240,245" stroke="{CYAN}"/>
    
    <!-- Mobile Outer Frame (Purple) -->
    <path d="M 330,280 L 330,380 C 330,400 340,410 360,410 L 412,410 C 432,410 442,400 442,380 L 442,140 C 442,120 432,110 412,110 L 380,110" stroke="{PURPLE}"/>
    
    <!-- Bridge Diagonal (N diagonal) connecting Monitor Top-Left to Mobile Bottom-Left area -->
    <path d="M 130,110 L 330,380" stroke="url(#bridgeGrad)"/>
    
    <!-- N Right Stem (Part of Mobile Left Edge) -->
    <path d="M 330,380 L 330,220" stroke="{PURPLE}"/>
</g>
</svg>'''
    return svg

def save_svg(path, content):
    with open(path, 'w') as f:
        f.write(content)

def main():
    svg_transparent = generate_svg(with_bg=False)
    svg_bg = generate_svg(with_bg=True)
    
    save_svg('logo_transparent.svg', svg_transparent)
    save_svg('logo_bg.svg', svg_bg)
    save_svg('logo.svg', svg_bg)
    
    res_dir = 'android/app/src/main/res'
    
    adaptive_dir = os.path.join(res_dir, 'mipmap-anydpi-v26')
    os.makedirs(adaptive_dir, exist_ok=True)
    ic_launcher_xml = """<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@drawable/ic_launcher_foreground"/>
    <monochrome android:drawable="@drawable/ic_launcher_foreground"/>
</adaptive-icon>"""
    with open(os.path.join(adaptive_dir, 'ic_launcher.xml'), 'w') as f:
        f.write(ic_launcher_xml)
    with open(os.path.join(adaptive_dir, 'ic_launcher_round.xml'), 'w') as f:
        f.write(ic_launcher_xml)
        
    values_dir = os.path.join(res_dir, 'values')
    os.makedirs(values_dir, exist_ok=True)
    colors_xml = """<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">#070B12</color>
</resources>"""
    with open(os.path.join(values_dir, 'ic_launcher_colors.xml'), 'w') as f:
        f.write(colors_xml)

    drawable_dir = os.path.join(res_dir, 'drawable')
    os.makedirs(drawable_dir, exist_ok=True)
    vector_xml = f"""<?xml version="1.0" encoding="utf-8"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp"
    android:height="108dp"
    android:viewportWidth="512"
    android:viewportHeight="512">
    
    <path
        android:strokeColor="{CYAN}"
        android:strokeWidth="{STROKE_WIDTH}"
        android:strokeLineCap="round"
        android:strokeLineJoin="round"
        android:pathData="M 100,380 L 100,140 C 100,120 110,110 130,110 L 300,110 C 320,110 330,120 330,140 L 330,180" />
        
    <path
        android:strokeColor="{CYAN}"
        android:strokeWidth="{STROKE_WIDTH}"
        android:strokeLineCap="round"
        android:strokeLineJoin="round"
        android:pathData="M 100,245 L 240,245" />
        
    <path
        android:strokeColor="{PURPLE}"
        android:strokeWidth="{STROKE_WIDTH}"
        android:strokeLineCap="round"
        android:strokeLineJoin="round"
        android:pathData="M 330,280 L 330,380 C 330,400 340,410 360,410 L 412,410 C 432,410 442,400 442,380 L 442,140 C 442,120 432,110 412,110 L 380,110" />
        
    <path
        android:strokeColor="#4A98FF"
        android:strokeWidth="{STROKE_WIDTH}"
        android:strokeLineCap="round"
        android:strokeLineJoin="round"
        android:pathData="M 130,110 L 330,380" />
        
    <path
        android:strokeColor="{PURPLE}"
        android:strokeWidth="{STROKE_WIDTH}"
        android:strokeLineCap="round"
        android:strokeLineJoin="round"
        android:pathData="M 330,380 L 330,220" />
</vector>"""
    with open(os.path.join(drawable_dir, 'ic_launcher_foreground.xml'), 'w') as f:
        f.write(vector_xml)

if __name__ == '__main__':
    main()
