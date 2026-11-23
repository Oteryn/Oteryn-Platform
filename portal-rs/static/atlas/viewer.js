'use strict';
(() => {
  const view = document.getElementById('atlas');
  if (!view) return;
  const data = JSON.parse(view.dataset.map);
  const canvas = view.querySelector('canvas'), ctx = canvas.getContext('2d');
  if (!ctx) return;
  const floor = document.getElementById('floor'), status = document.getElementById('status');
  const tiles = document.getElementById('tiles'), markers = document.getElementById('markers');
  const tileStatus = document.getElementById('tile-status');
  const list = document.getElementById('entities');
  const zoomIn = document.getElementById('in'), zoomOut = document.getElementById('out');
  const cache = new Map();
  let markerFloor = null, markersShown = false;
  let x = data.x, y = data.y, z = data.floor, zoom = data.zoom;
  for (const m of data.manifests) {
    const option = document.createElement('option');
    option.value = m.floor; option.textContent = m.floor; floor.append(option);
  }
  floor.value = z;
  let overlay = () => {};
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
    const updateMarkerList = markerFloor !== z || markersShown !== markers.checked;
    if (updateMarkerList) {
      if (list.contains(document.activeElement)) markers.focus();
      list.replaceChildren();
      markerFloor = z;
      markersShown = markers.checked;
    }
    if (markers.checked) for (const marker of data.markers.filter(mk => mk.floor === z)) {
      ctx.fillStyle = '#ffdc78'; ctx.beginPath();
      ctx.arc((marker.x-m.bounds.min_x)/scale-left,(marker.y-m.bounds.min_y)/scale-top,6,0,2*Math.PI); ctx.fill();
      if (updateMarkerList) {
        const li = document.createElement('li'), link = document.createElement('a');
        link.href = `/${document.documentElement.lang}/wiki/npcs/${encodeURIComponent(marker.entity_key)}`;
        link.textContent = `${marker.entity_key} (${marker.x}, ${marker.y}, ${marker.floor})`;
        li.append(link); list.append(li);
      }
    }
    announce(status, status.dataset.position.replace(/%(x|y|floor|zoom)/g,
      (_, key) => ({x, y, floor: z, zoom})[key]));
    announce(tileStatus, missing ? tileStatus.dataset.missing : '');
    const url = new URL(location.href);
    for (const [key,value] of Object.entries({x,y,z,zoom})) url.searchParams.set(key,value);
    if (url.href !== location.href) history.replaceState(null, '', url);
    overlay({x,y,z,zoom,scale,left,top,m,canvas,ctx});
    zoomIn.setAttribute('aria-disabled', zoom === m.levels.at(-1).z);
    zoomOut.setAttribute('aria-disabled', zoom === m.levels[0].z);
  }
  function move(dx,dy) { const scale=2**(manifest().levels.at(-1).z-zoom); x+=dx*scale; y+=dy*scale; draw(); }
  function zoomBy(button, step) {
    if (button.getAttribute('aria-disabled') === 'true') return;
    zoom += step;
    draw();
  }
  zoomIn.onclick = () => zoomBy(zoomIn, 1);
  zoomOut.onclick = () => zoomBy(zoomOut, -1);
  for (const [id,dx,dy] of [['north',0,-64],['south',0,64],['west',-64,0],['east',64,0]]) document.getElementById(id).onclick = () => move(dx,dy);
  floor.onchange = () => { z=Number(floor.value); draw(); };
  tiles.onchange = markers.onchange = draw;
  view.onkeydown = event => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const actions = {ArrowLeft:()=>move(-64,0),ArrowRight:()=>move(64,0),ArrowUp:()=>move(0,-64),ArrowDown:()=>move(0,64),'+':()=>zoomBy(zoomIn,1),'-':()=>zoomBy(zoomOut,-1)};
    if (actions[event.key]) { event.preventDefault(); actions[event.key](); }
  };
  let drag;
  view.onpointerdown = event => { drag={x:event.clientX,y:event.clientY}; view.setPointerCapture(event.pointerId); view.focus(); };
  view.onpointermove = event => { if(drag) { const dx=drag.x-event.clientX,dy=drag.y-event.clientY; drag={x:event.clientX,y:event.clientY}; move(dx,dy); } };
  view.onpointerup = view.onpointercancel = () => { drag=null; };
  new ResizeObserver(draw).observe(view);
  draw();
  atlasSpawns(draw, fn => { overlay=fn; draw(); });
})();

function announce(node, value) {
  if (node.textContent !== value) node.textContent = value;
}

// Uses the shared viewer coordinate frame and its keyboard navigation.
async function atlasSpawns(redraw, install) {
  const lang = document.documentElement.lang;
  let text;
  try {
    const response = await fetch(`/atlas/spawns-text/${lang}`, {cache:'no-store'});
    if (!response.ok) return;
    text = await response.json();
  } catch { return; }
  const element = (tag, content) => {
    const node = document.createElement(tag);
    if (content) node.textContent = content;
    return node;
  };
  const button = (key, action) => {
    const node = element('button',text[key]); node.type='button'; node.onclick=action; return node;
  };
  const panel=element('section'), heading=element('h2',text.spawns);
  const viewer = document.getElementById('atlas-viewer');
  heading.id = 'spawn-layer-title';
  panel.setAttribute('aria-labelledby', heading.id);
  panel.className='spawn-panel'; panel.append(heading); viewer.append(panel);
  const toggleLabel=element('label',text.spawns), toggle=element('input');
  toggle.type='checkbox'; toggle.checked=true; toggleLabel.prepend(toggle);
  const place=element('a',text.place);
  const controls=element('nav');
  controls.setAttribute('aria-labelledby', heading.id);
  if (document.fullscreenEnabled) {
    const fullscreen = button('fullscreen', async () => {
      try {
        if (document.fullscreenElement === viewer) await document.exitFullscreen();
        else await viewer.requestFullscreen();
      } catch { announce(outcome, text.failed); }
    });
    document.addEventListener('fullscreenchange', () => {
      fullscreen.textContent = document.fullscreenElement === viewer ? text['exit-fullscreen'] : text.fullscreen;
      fullscreen.focus();
    });
    controls.append(fullscreen);
  }
  controls.append(toggleLabel, place);
  // Spawn loading owns `notice`; marker and report outcomes own `outcome`, so a late spawn load cannot erase them.
  const count=element('p'), notice=element('p'), outcome=element('p'), list=element('ul');
  notice.setAttribute('role','status'); count.setAttribute('role','status'); outcome.setAttribute('role','status');
  panel.append(controls,count,notice,outcome,list);
  let state, points=[], privateMarkers=[], csrf=null, timer, controller, requestKey='', loadedKey='';
  let listed = [], listedZoom;
  toggle.onchange=() => {
    if (!toggle.checked && loadedKey !== requestKey) {
      clearTimeout(timer);
      controller?.abort();
      requestKey = '';
      announce(notice, '');
    }
    redraw();
  };
  function link(cell, label) {
    const a=element('a',label), url=new URL(location.href);
    for(const [key,value] of Object.entries({x:cell.x,y:cell.y,z:cell.floor,zoom:state.zoom})) url.searchParams.set(key,value);
    a.href=url.pathname+url.search; return a;
  }
  function visible(cell) {
    if(cell.floor!==state.z) return false;
    const px=(cell.x-state.m.bounds.min_x)/state.scale-state.left;
    const py=(cell.y-state.m.bounds.min_y)/state.scale-state.top;
    return px>=0 && py>=0 && px<state.canvas.width && py<state.canvas.height;
  }
  function dot(cell, label, colour) {
    const px=(cell.x-state.m.bounds.min_x)/state.scale-state.left;
    const py=(cell.y-state.m.bounds.min_y)/state.scale-state.top;
    state.ctx.fillStyle=colour; state.ctx.beginPath(); state.ctx.arc(px,py,10,0,2*Math.PI); state.ctx.fill();
    state.ctx.fillStyle='#111820'; state.ctx.font='bold 12px system-ui'; state.ctx.textAlign='center'; state.ctx.fillText(label,px,py+4);
  }
  function requestSpawns() {
    const b=state.m.bounds, clamp=n=>Math.max(0,Math.min(65535,Math.floor(n)));
    const params=new URLSearchParams({version:state.m.world_version,floor:state.z,
      min_x:clamp(Math.max(b.min_x,b.min_x+state.left*state.scale)),
      min_y:clamp(Math.max(b.min_y,b.min_y+state.top*state.scale)),
      max_x:clamp(Math.min(b.max_x-1,b.min_x+(state.left+state.canvas.width)*state.scale)),
      max_y:clamp(Math.min(b.max_y-1,b.min_y+(state.top+state.canvas.height)*state.scale))});
    const key=params.toString();
    if(key===requestKey) return;
    requestKey=key; loadedKey=''; points=[];
    clearTimeout(timer); if(controller) controller.abort();
    announce(notice, text.loading);
    timer=setTimeout(async () => {
      const request = new AbortController();
      controller = request;
      try {
        const response=await fetch(`/atlas/spawns?${key}`, {signal:request.signal,cache:'no-store'});
        if(!response.ok) throw new Error('unavailable');
        const result=await response.json();
        if(request.signal.aborted) return;
        points=result; loadedKey=key; announce(notice, ''); redraw();
      } catch {
        if (!request.signal.aborted) {
          points=[]; announce(notice, text['spawns-unavailable']); redraw();
        }
      }
    },180);
  }
  install(current => {
    state=current; place.href=location.href;
    if (toggle.checked) requestSpawns();
    const shown=toggle.checked ? points.filter(p=>visible(p.cell)) : [];
    announce(count, toggle.checked && loadedKey===requestKey
      ? `${text.monsters}: ${shown.length}; ${text.sites}: ${new Set(shown.map(p=>p.spawn)).size}` : '');
    const updateList = listedZoom !== state.zoom || shown.length !== listed.length
      || shown.some((point, index) => point !== listed[index]);
    if (updateList) {
      if (list.contains(document.activeElement)) toggle.focus();
      list.replaceChildren();
      listed = shown;
      listedZoom = state.zoom;
    }
    const groups=new Map();
    for(const point of shown) {
      // Grid in screen pixels: zooming in separates groups into individual points.
      const size=state.scale>1 ? 32*state.scale : 1;
      const key=`${Math.floor(point.cell.x/size)},${Math.floor(point.cell.y/size)}`;
      if(!groups.has(key)) groups.set(key,[]); groups.get(key).push(point);
    }
    for(const group of groups.values()) {
      const cell=group[0].cell; dot(cell,String(group.length),'#ffdc78');
      if (!updateList) continue;
      const item=element('li');
      item.append(link(cell,`${text.monsters}: ${group.length}; ${text.sites}: ${new Set(group.map(p=>p.spawn)).size} (${cell.x}, ${cell.y}, ${cell.floor})`));
      const details=element('details'), summary=element('summary',text.spawns); details.append(summary);
      for(const point of group) {
        const row=element('p'), wiki=element('a',point.creature);
        wiki.href=`/${lang}/wiki/creatures/${encodeURIComponent(point.creature)}`;
        const respawn = point.respawn_ms == null
          ? text['no-ordinary-respawn']
          : `${text.respawn}: ${point.respawn_ms / 1000}`;
        row.append(wiki,document.createTextNode(`; ${respawn}; `),link(point.cell,`${point.cell.x}, ${point.cell.y}, ${point.cell.floor}`));
        details.append(row);
      }
      item.append(details); list.append(item);
    }
    for(const marker of privateMarkers.filter(visible)) dot(marker,'◆','#a7dcff');
  });
  const privateTitle=element('h2',text.private), privateList=element('ul');
  privateTitle.id='private-markers-title';
  privateTitle.tabIndex=-1;
  privateList.setAttribute('aria-labelledby', privateTitle.id);

  function appendPrivate(marker) {
    const item=element('li'), waypoint=link(marker,marker.label);
    waypoint.id=`private-marker-${marker.id}`;
    const remove=button('remove',async()=>{
      if (remove.getAttribute('aria-disabled') === 'true') return;
      remove.setAttribute('aria-disabled', 'true');
      announce(outcome, '');
      try {
        await mutate(`/atlas/private-markers/${marker.id}`,'DELETE');
        if (item.contains(document.activeElement)) {
          const neighbor=item.nextElementSibling || item.previousElementSibling;
          (neighbor?.querySelector('button') || privateTitle).focus();
        }
        item.remove();
        privateMarkers=privateMarkers.filter(entry=>entry.id!==marker.id);
        redraw();
        announce(outcome, text.saved);
      } catch {
        remove.removeAttribute('aria-disabled');
        announce(outcome, text.failed);
      }
    });
    remove.setAttribute('aria-describedby', waypoint.id);
    item.append(waypoint,remove);
    privateList.append(item);
  }
  async function mutate(url,method,body) {
    const response=await fetch(url,{method,headers:{'Content-Type':'application/json','x-csrf-token':csrf},body:body ? JSON.stringify(body) : undefined});
    if(!response.ok) throw new Error('unavailable');
    return response;
  }
  try {
    const response=await fetch('/atlas/private-markers',{cache:'no-store'});
    if (response.status===401) {
      panel.append(element('p',text['sign-in']));
      return;
    }
    if (!response.ok) throw new Error('unavailable');
    const data=await response.json();
    csrf=data.csrf_token;
    privateMarkers=data.markers;
  } catch { announce(outcome, text.failed); return; }
  panel.append(privateTitle,privateList);
  privateMarkers.forEach(appendPrivate);
  redraw();

  function form(key, submit, min, max, url, onSaved) {
    const form=element('form'), label=element('label',text[key]), input=element('textarea');
    input.required=true; input.minLength=min; input.maxLength=max; label.append(input);
    const send=element('button',text[submit]); send.type='submit'; form.append(label,send);
    form.onsubmit=async event=>{
      event.preventDefault();
      if (send.getAttribute('aria-disabled') === 'true') return;
      send.setAttribute('aria-disabled','true');
      announce(outcome, '');
      try {
        const response = await mutate(url,'POST',{x:state.x,y:state.y,floor:state.z,text:input.value});
        if (onSaved) await onSaved(response);
        input.value='';
        announce(outcome, text.saved);
      } catch { announce(outcome, text.failed); } finally { send.removeAttribute('aria-disabled'); }
    }; panel.append(form);
  }
  form('label','save',1,80,'/atlas/private-markers',async response=>{
    const marker=await response.json();
    privateMarkers.push(marker);
    appendPrivate(marker);
    redraw();
  });
  panel.append(element('h2',text.report));
  form('description','send',10,2000,'/atlas/map-reports');
}
// Owner-only hunting-place layer. Labels come from the server i18n layer.
(() => {
  const atlas = document.getElementById('atlas');
  const lang = document.documentElement.lang;
  if (!atlas || !['pl', 'en'].includes(lang)) return;
  fetch(`/${lang}/atlas/places`, { credentials: 'same-origin', cache: 'no-store' })
    .then(response => response.ok ? response.text() : null)
    .then(markup => {
      if (!markup) return;
      const page = new DOMParser().parseFromString(markup, 'text/html');
      const heading = page.querySelector('h1');
      if (!heading) return;
      const link = document.createElement('a');
      link.href = `/${lang}/atlas/places`;
      link.textContent = heading.textContent;
      const layer = document.createElement('details');
      layer.className = 'hunting-places-layer';
      const summary = document.createElement('summary');
      summary.textContent = heading.textContent;
      layer.append(summary, link);
      const help = page.querySelector('main > p');
      if (help) {
        const description = document.createElement('p');
        description.textContent = help.textContent;
        layer.append(description);
      }
      const rows = page.querySelector('ul.places');
      if (rows) layer.append(document.importNode(rows, true));
      document.getElementById('atlas-viewer').append(layer);
    }).catch(() => {});
})();
