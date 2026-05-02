const { Client } = require('ssh2');

const conn = new Client();

conn.on('ready', () => {
  console.log('Connected!');
  const commands = [
    'cd /root/claudia-server && git pull origin main 2>&1',
    'source $HOME/.cargo/env && cd /root/claudia-server/server && cargo build --release 2>&1'
  ];

  let idx = 0;
  function runNext() {
    if (idx >= commands.length) { conn.end(); return; }
    const cmd = commands[idx++];
    console.log('\n>>> Running:', cmd, '\n');
    conn.exec(cmd, (err, stream) => {
      if (err) { console.error(err); conn.end(); return; }
      stream.on('data', (d) => process.stdout.write(d.toString()));
      stream.on('close', () => { console.log('\n--- Done ---\n'); runNext(); });
    });
  }
  runNext();
});

conn.connect({ host: '65.108.216.215', port: 22, username: 'root', password: 'Da@7355608###' });