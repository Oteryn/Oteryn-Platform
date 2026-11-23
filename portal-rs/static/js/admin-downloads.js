const releaseForm = document.querySelector('#signed-release');

function encodeReleaseFile(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener('load', () => resolve(reader.result.split(',', 2)[1]));
    reader.addEventListener('error', reject);
    reader.addEventListener('abort', reject);
    reader.readAsDataURL(file);
  });
}

if (releaseForm) {
  const submit = releaseForm.querySelector('button');
  const feedback = releaseForm.querySelector('[role=alert]');
  submit.disabled = false;
  releaseForm.addEventListener('submit', async event => {
    event.preventDefault();
    submit.disabled = true;
    feedback.textContent = '';
    try {
      const fields = releaseForm.elements;
      const [policy] = fields.policy.files;
      const files = [...fields.metadata.files];
      const mib = 1024 * 1024;
      if (!policy || !policy.size || policy.size > mib / 4 || !files.length || files.length > 32
        || files.some(file => !file.size || file.size > mib)
        || files.reduce((size, file) => size + file.size, 0) > 8 * mib) {
        feedback.textContent = releaseForm.dataset.invalid;
        return;
      }
      const metadata = [];
      for (const file of files) {
        metadata.push({ name: file.name, content: await encodeReleaseFile(file) });
      }
      const response = await fetch(releaseForm.action, {
        method: 'POST',
        redirect: 'manual',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': fields._csrf.value },
        body: JSON.stringify({ channel: fields.channel.value, policy: await encodeReleaseFile(policy), metadata }),
      });
      if (response.status === 201) {
        const { id } = await response.json();
        const destination = new URL(releaseForm.action);
        destination.searchParams.set('publication', id);
        destination.hash = 'publication-' + id;
        location.assign(destination);
        return;
      }
      const result = await response.json().catch(() => ({}));
      const key = response.status === 403 || response.type === 'opaqueredirect' ? 'denied'
        : response.status === 413 ? 'invalid' : result.error;
      feedback.textContent = releaseForm.dataset[key] || releaseForm.dataset.unavailable;
    } catch {
      feedback.textContent = releaseForm.dataset.unavailable;
    } finally {
      submit.disabled = false;
    }
  });
}
