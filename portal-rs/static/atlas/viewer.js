'use strict';
(() => {
  const view = document.getElementById('atlas');
  if (!view) return;
  const data = JSON.parse(view.dataset.map);
  const canvas = view.querySelector('canvas'), ctx = canvas.getContext('2d');
  if (!ctx) return;
  const floor = document.getElementById('floor'), status = document.getElementById('status');
  const tiles = document.getElementById('tiles'), markers = document.getElementById('markers');
  const cache = new Map();
  let x = data.x, y = data.y, z = data.floor, zoom = data.zoom;
  for (const m of data.manifests) {
    const option = document.createElement('option');
    option.value = m.floor; option.textContent = m.floor; floor.append(option);
  }
  floor.value = z;
  const manifest = () => data.manifests.find(m => m.floor === z);
  function draw() {
    const m = manifest();
    zoom = Math.max(m.levels[0].z, Math.min(zoom, m.levels.at(-1).z));
    x = Math.max(m.bounds.min_x, Math.min(Math.round(x), m.bounds.max_x - 1));
    y = Math.max(m.bounds.min_y, Math.min(Math.round(y), m.bounds.max_y - 1));
    const level = m.levels.find(l => l.z === zoom);
    const scale = 2 ** (m.levels.at(-1).z - zoom);
    canvas.width = view.clientWidth; canvas.height = view.clientHeight;
    ctx.fillStyle = '#111820'; ctx.fillRect(0,0,canvas.width,canvas.height);
    const left = (x-m.bounds.min_x)/scale-canvas.width/2;
    const top = (y-m.bounds.min_y)/scale-canvas.height/2;
    let missing = false;
    if (tiles.checked) {
      for (let tx=Math.max(0,Math.floor(left/256));tx<Math.min(level.columns,Math.ceil((left+canvas.width)/256));tx++) {
        for (let ty=Math.max(0,Math.floor(top/256));ty<Math.min(level.rows,Math.ceil((top+canvas.height)/256));ty++) {
          const url = `/atlas/tiles/${encodeURIComponent(m.world_version)}/${z}/${zoom}/${tx}/${ty}.webp`;
          if (!cache.has(url)) {
            const image = new Image(); cache.set(url,image);
            image.onload = draw; image.onerror = () => { cache.set(url,null); draw(); }; image.src = url;
          }
          const image = cache.get(url);
          if (image && image.complete && image.naturalWidth) ctx.drawImage(image,tx*256-left,ty*256-top);
          if (image === null) missing = true;
        }
      }
    }
    const list = document.getElementById('entities'); list.replaceChildren();
    if (markers.checked) for (const marker of data.markers.filter(mk => mk.floor === z)) {
      ctx.fillStyle = '#ffdc78'; ctx.beginPath();
      ctx.arc((marker.x-m.bounds.min_x)/scale-left,(marker.y-m.bounds.min_y)/scale-top,6,0,2*Math.PI); ctx.fill();
      const li = document.createElement('li'), link = document.createElement('a');
      link.href = `/${document.documentElement.lang}/wiki/npcs/${encodeURIComponent(marker.entity_key)}`;
      link.textContent = `${marker.entity_key} (${marker.x}, ${marker.y}, ${marker.floor})`;
      li.append(link); list.append(li);
    }
    status.textContent = missing ? status.dataset.missing : `${x}, ${y}, ${z}`;
    const url = new URL(location.href);
    for (const [key,value] of Object.entries({x,y,z,zoom})) url.searchParams.set(key,value);
    history.replaceState(null,'',url);
    document.getElementById('in').disabled = zoom === m.levels.at(-1).z;
    document.getElementById('out').disabled = zoom === m.levels[0].z;
  }
  function move(dx,dy) { const scale=2**(manifest().levels.at(-1).z-zoom); x+=dx*scale; y+=dy*scale; draw(); }
  document.getElementById('in').onclick = () => { zoom++; draw(); };
  document.getElementById('out').onclick = () => { zoom--; draw(); };
  for (const [id,dx,dy] of [['north',0,-64],['south',0,64],['west',-64,0],['east',64,0]]) document.getElementById(id).onclick = () => move(dx,dy);
  floor.onchange = () => { z=Number(floor.value); draw(); };
  tiles.onchange = markers.onchange = draw;
  view.onkeydown = event => {
    const actions = {ArrowLeft:()=>move(-64,0),ArrowRight:()=>move(64,0),ArrowUp:()=>move(0,-64),ArrowDown:()=>move(0,64),'+':()=>{zoom++;draw();},'-':()=>{zoom--;draw();}};
    if (actions[event.key]) { event.preventDefault(); actions[event.key](); }
  };
  let drag;
  view.onpointerdown = event => { drag={x:event.clientX,y:event.clientY}; view.setPointerCapture(event.pointerId); view.focus(); };
  view.onpointermove = event => { if(drag) { const dx=drag.x-event.clientX,dy=drag.y-event.clientY; drag={x:event.clientX,y:event.clientY}; move(dx,dy); } };
  view.onpointerup = view.onpointercancel = () => { drag=null; };
  new ResizeObserver(draw).observe(view);
  draw();
})();
