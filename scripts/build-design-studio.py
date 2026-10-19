"""Build a standalone extension JSON from editable local sources. No network."""
from pathlib import Path
import argparse,json,html

folder=Path(__file__).resolve().parent
root=folder.parent
standalone=(folder/'src').is_dir()
parser=argparse.ArgumentParser();parser.add_argument('--source',type=Path,default=folder/'src' if standalone else root/'extensions/design-studio');parser.add_argument('--output',type=Path,default=folder/'design-studio.toolhub-extension.json' if standalone else root/'extensions/examples/design-studio.toolhub-extension.json');options=parser.parse_args()
source=options.source
package=json.loads((source/'manifest.json').read_text(encoding='utf-8'))
preview_css=(source/'preview.css').read_text(encoding='utf-8')
view=package['views'][0]
view['html']=(source/'view.html').read_text(encoding='utf-8')+'\n<template id="design-preview-css">'+html.escape(preview_css,quote=False)+'</template>'
view['css']=preview_css+'\n'+(source/'view.css').read_text(encoding='utf-8')
view['script']='globalThis.DESIGN_PREVIEW_CSS=document.getElementById("design-preview-css").content.textContent;\n'+(source/'preview.js').read_text(encoding='utf-8')+'\n'+(source/'view.js').read_text(encoding='utf-8')
for name,maximum in [('html',65536),('css',32768),('script',65536)]:
    size=len(view[name].encode('utf-8'))
    assert size<=maximum,(name,size,maximum)
data=(json.dumps(package,ensure_ascii=False,indent=2)+'\n').encode('utf-8')
assert len(data)<=262144
destination=options.output
destination.write_bytes(data)
print(json.dumps({'bytes':len(data),'html':len(view['html'].encode()),'css':len(view['css'].encode()),'script':len(view['script'].encode())}))
