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
  atlasSpawns(view, draw, fn => { overlay=fn; draw(); });
})();

// Uses the shared viewer coordinate frame and its keyboard navigation.
async function atlasSpawns(view, redraw, install) {
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
  panel.className='spawn-panel'; panel.append(heading); view.after(panel);
  const toggleLabel=element('label',text.spawns), toggle=element('input');
  toggle.type='checkbox'; toggle.checked=true; toggleLabel.prepend(toggle);
  const place=element('a',text.place);
  const controls=element('nav'); controls.append(toggleLabel, place, button('fullscreen',async () => {
    try { if(document.fullscreenElement) await document.exitFullscreen(); else await view.requestFullscreen(); } catch { notice.textContent=text.failed; }
  }));
  const count=element('p'), notice=element('p'), list=element('ul');
  notice.setAttribute('role','status'); count.setAttribute('role','status');
  panel.append(controls,count,notice,list);
  let state, points=[], privateMarkers=[], csrf=null, timer, controller, requestKey='', loadedKey='';
  toggle.onchange=redraw;
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
    notice.textContent=text.loading;
    timer=setTimeout(async () => {
      controller=new AbortController();
      try {
        const response=await fetch(`/atlas/spawns?${key}`, {signal:controller.signal,cache:'no-store'});
        if(!response.ok) throw new Error('unavailable');
        const result=await response.json();
        if(requestKey!==key) return;
        points=result; loadedKey=key; notice.textContent=''; redraw();
      } catch(error) { if(error.name!=='AbortError' && requestKey===key) { points=[]; notice.textContent=text.failed; redraw(); } }
    },180);
  }
  install(current => {
    state=current; place.href=location.href; requestSpawns(); list.replaceChildren();
    const shown=toggle.checked ? points.filter(p=>visible(p.cell)) : [];
    count.textContent=loadedKey===requestKey ? `${text.monsters}: ${shown.length}; ${text.sites}: ${new Set(shown.map(p=>p.spawn)).size}` : text.loading;
    const groups=new Map();
    for(const point of shown) {
      // Grid in screen pixels: zooming in separates groups into individual points.
      const size=state.scale>1 ? 32*state.scale : 1;
      const key=`${Math.floor(point.cell.x/size)},${Math.floor(point.cell.y/size)}`;
      if(!groups.has(key)) groups.set(key,[]); groups.get(key).push(point);
    }
    for(const group of groups.values()) {
      const cell=group[0].cell; dot(cell,String(group.length),'#ffdc78');
      const item=element('li');
      item.append(link(cell,`${text.monsters}: ${group.length}; ${text.sites}: ${new Set(group.map(p=>p.spawn)).size} (${cell.x}, ${cell.y}, ${cell.floor})`));
      const details=element('details'), summary=element('summary',text.spawns); details.append(summary);
      for(const point of group) {
        const row=element('p'), wiki=element('a',point.creature);
        wiki.href=`/${lang}/wiki/creatures/${encodeURIComponent(point.creature)}`;
        row.append(wiki,document.createTextNode(`; ${text.respawn}: ${point.respawn_ms/1000}; `),link(point.cell,`${point.cell.x}, ${point.cell.y}, ${point.cell.floor}`));
        details.append(row);
      }
      item.append(details); list.append(item);
    }
    for(const marker of privateMarkers.filter(visible)) dot(marker,'◆','#a7dcff');
  });
  const privateTitle=element('h2',text.private), privateList=element('ul'); panel.append(privateTitle,privateList);
  function renderPrivate() {
    privateList.replaceChildren();
    for(const marker of privateMarkers) {
      const item=element('li'); item.append(link(marker,marker.label),button('remove',async()=>{
        try { await mutate(`/atlas/private-markers/${marker.id}`,'DELETE'); await loadPrivate(); notice.textContent=text.saved; } catch { notice.textContent=text.failed; }
      })); privateList.append(item);
    }
    redraw();
  }
  async function loadPrivate() {
    const response=await fetch('/atlas/private-markers',{cache:'no-store'});
    if(!response.ok) throw new Error('unavailable');
    const data=await response.json(); csrf=data.csrf_token; privateMarkers=data.markers; renderPrivate();
  }
  async function mutate(url,method,body) {
    const response=await fetch(url,{method,headers:{'Content-Type':'application/json','x-csrf-token':csrf},body:body ? JSON.stringify(body) : undefined});
    if(!response.ok) throw new Error('unavailable');
  }
  try { await loadPrivate(); } catch { panel.append(element('p',text['sign-in'])); return; }
  function form(key, submit, min, max, url) {
    const form=element('form'), label=element('label',text[key]), input=element('textarea');
    input.required=true; input.minLength=min; input.maxLength=max; label.append(input);
    const send=element('button',text[submit]); send.type='submit'; form.append(label,send);
    form.onsubmit=async event=>{
      event.preventDefault(); send.disabled=true;
      try {
        await mutate(url,'POST',{x:state.x,y:state.y,floor:state.z,text:input.value});
        input.value=''; notice.textContent=text.saved; await loadPrivate();
      } catch { notice.textContent=text.failed; } finally { send.disabled=false; }
    }; panel.append(form);
  }
  form('label','save',1,80,'/atlas/private-markers');
  panel.append(element('h2',text.report)); form('description','send',10,2000,'/atlas/map-reports');
}
