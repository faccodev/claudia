const { Client } = require('ssh2');

const conn = new Client();

conn.on('ready', () => {
  console.log('Connected! Pulling code...');
  conn.exec('cd /root/claudia-server && git pull origin main 2>&1', (err, stream) => {
    if (err) { console.error(err); conn.end(); return; }
    stream.on('data', (d) => process.stdout.write(d.toString()));
    stream.on('close', () => {
      console.log('\n--- Now building ---');
      conn.exec('source $HOME/.cargo/env && cd /root/claudia-server/server && cargo build --release 2>&1', (err, stream) => {
        if (err) { console.error(err); conn.end(); return; }
        stream.on('data', (d) => process.stdout.write(d.toString()));
        stream.on('close', (code) => {
          console.log('\n--- Build complete with code', code, '---');
          conn.end();
        });
      });
    });
  });
});

conn.connect({ host: '65.108.216.215', port: 22, username: 'root', password: 'Da@7355608###' });