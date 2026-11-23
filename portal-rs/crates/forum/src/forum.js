const readMarker = document.querySelector('form[data-forum-read]');
if (readMarker) {
  const mark = async () => {
    if (document.visibilityState !== 'visible') return;
    document.removeEventListener('visibilitychange', mark);
    try {
      const response = await fetch(readMarker.action, {
        method: 'POST',
        body: new URLSearchParams(new FormData(readMarker)),
        headers: {'X-Forum-Read': '1'},
      });
      if (response.ok) readMarker.hidden = true;
    } catch {
      // The regular form remains available if the background request fails.
    }
  };
  document.addEventListener('visibilitychange', mark);
  void mark();
}
