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
