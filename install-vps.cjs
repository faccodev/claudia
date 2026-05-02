const { Client } = require('ssh2');

const VPS_HOST = '65.108.216.215';
const ROOT_PASS = 'Da@7355608###';

async function main() {
  console.log('=== Claudia Installation ===\n');

  const conn = new Client();

  await new Promise((resolve, reject) => {
    conn.connect({ host: VPS_HOST, port: 22, username: 'root', password: ROOT_PASS });
    conn.on('ready', resolve);
    conn.on('error', reject);
  });
  console.log('Connected!');

  await new Promise((resolve, reject) => {
    conn.shell({ term: 'vt100', cols: 200, rows: 80 }, async (err, stream) => {
      if (err) { reject(err); return; }

      let output = '';
      let phase = 0;

      const send = (text) => stream.write(text);

      stream.on('data', (data) => {
        const text = data.toString();
        output += text;
        process.stdout.write(text);

        if (text.includes('#') && phase === 0) {
          phase = 1;
          console.log('\n*** SHELL READY ***\n');

          setTimeout(() => send('useradd -m -s /bin/bash danilo 2>/dev/null || echo "exists"\n'), 300);
          setTimeout(() => send("echo 'danilo:Da@7355608###' | chpasswd\n"), 600);
          setTimeout(() => send('id danilo\n'), 900);
          setTimeout(() => send('usermod -aG sudo danilo\n'), 1200);
          setTimeout(() => send('apt-get update -qq 2>&1 | tail -5\n'), 2000);
          setTimeout(() => send('DEBIAN_FRONTEND=noninteractive apt-get install -y curl wget git build-essential pkg-config libssl-dev 2>&1 | tail -5\n'), 3000);
          setTimeout(() => {
            console.log('\n>>> Running Claudia install (this takes a while - Rust build)...\n');
            const cmd = `curl -sL https://raw.githubusercontent.com/faccodev/claudia/main/server/install.sh | bash -s -- --domain claudia.facco.dev --api-key sk-ant-api03-xxxxx --admin-user danilo --admin-password Da@7355608### --skip-ssl 2>&1`;
            send(cmd + '\n');
          }, 5000);
          setTimeout(() => send('systemctl status claudia-server 2>&1 | head -10\n'), 120000);
          setTimeout(() => send('journalctl -u claudia-server -n 20 --no-pager\n'), 125000);
        }
      });

      stream.on('close', () => resolve());

      // Extended timeout for Rust compilation
      setTimeout(() => {
        console.log('[Timeout - closing]');
        stream.end('exit\n');
      }, 130000);
    });
  });

  conn.end();
  console.log('\n=== COMPLETE ===');
}

main().catch(console.error);