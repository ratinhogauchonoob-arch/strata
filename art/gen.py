# Strata mark: five offset bars that read as an "S".
BARS=[(14,56),(8,30),(14,50),(34,56),(8,50)]
def bars(ox,oy,s,h,gap,colors,rx=None):
    out=[]
    for i,(a,b) in enumerate(BARS):
        y=oy+i*(h+gap)*s
        out.append(f'<rect x="{ox+a*s:.1f}" y="{y:.1f}" width="{(b-a)*s:.1f}" height="{h*s:.1f}" rx="{(rx if rx is not None else h/2)*s:.1f}" fill="{colors[i%len(colors)]}"/>')
    return "".join(out)
EMBER="#FF7A2F"; COBALT="#5163F5"; FOG="#E9EBEF"
cols=[EMBER,COBALT,FOG,COBALT,EMBER]
icon=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#1A1C21"/><stop offset="1" stop-color="#0B0C0E"/></linearGradient></defs>
<rect x="24" y="24" width="464" height="464" rx="104" fill="url(#bg)"/>
<rect x="25.5" y="25.5" width="461" height="461" rx="102.5" fill="none" stroke="#FFFFFF" stroke-opacity="0.07" stroke-width="3"/>
{bars(256-32*6.0,256-((5*9+4*3.5)*6.0)/2,6.0,9,3.5,cols)}
</svg>'''
open("icon.svg","w").write(icon)
# small icon: bolder, fewer colors
small=icon.replace('stroke-opacity="0.07"','stroke-opacity="0"')
open("icon-small.svg","w").write(small)
mark=f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">{bars(0,2.5,1,9,3.5,["var(--a1)","var(--a2)","var(--fg)","var(--a2)","var(--a1)"])}</svg>'
open("mark.svg","w").write(mark)
