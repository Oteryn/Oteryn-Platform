// Both launchers exercise the real SMTP clients against a loopback TLS receiver.
export function mailSinkOptions({ tls, secure, username, password, messages }) {
  return {
    ...tls,
    secure,
    onAuth(auth, _session, callback) {
      if (auth.username !== username || auth.password !== password) {
        callback(new Error('Invalid local SMTP credentials'));
        return;
      }
      callback(null, { user: username });
    },
    onData(stream, session, callback) {
      let raw = '';
      stream.on('data', chunk => { raw += chunk; });
      stream.on('end', () => {
        messages.push({ to: session.envelope.rcptTo.map(recipient => recipient.address), raw });
        callback();
      });
    },
  };
}
