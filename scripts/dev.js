const { spawn } = require('child_process');

const RESET = '\x1b[0m';
const COLORS = [
  '\x1b[36m', // Cyan     (Gateway)
  '\x1b[32m', // Green    (Identity)
  '\x1b[33m', // Yellow   (Auth)
  '\x1b[35m', // Magenta  (Email)
  '\x1b[34m', // Blue     (Human Verif)
  '\x1b[90m', // Gray     (Access Tokens)
  '\x1b[31m', // Red      (React)
];

const services = [
  { name: 'Gateway', cmd: 'cargo run --bin your_app_gateway --features local-dev', cwd: './backend-services' },
  { name: 'Identity', cmd: 'cargo run --bin your_app_identity --features local-dev', cwd: './backend-services' },
  { name: 'Auth', cmd: 'cargo run --bin your_app_auth --features local-dev', cwd: './backend-services' },
  { name: 'Email', cmd: 'cargo run --bin your_app_email --features local-dev', cwd: './backend-services' },
  { name: 'Verification', cmd: 'cargo run --bin your_app_human_verification --features local-dev', cwd: './backend-services' },
  { name: 'Tokens', cmd: 'cargo run --bin your_app_access_tokens --features local-dev', cwd: './backend-services' },
  { name: 'React', cmd: 'pnpm dev --mode development', cwd: './website' },
];

console.log('🚀 Booting YourApp Local Development Cluster...\n');

const children = [];

services.forEach((service, index) => {
  const color = COLORS[index % COLORS.length];
  // Pad names so the console output aligns perfectly
  const paddedName = service.name.padEnd(12, ' '); 
  const prefix = `${color}[${paddedName}]${RESET} | `;

  // Use shell: true to handle cross-platform execution (Windows cmd.exe compatibility)
  const child = spawn(service.cmd, {
    cwd: service.cwd,
    shell: true,
    stdio: ['ignore', 'pipe', 'pipe'] // Ignore stdin, pipe out/err so we can prefix lines
  });

  const log = (data) => {
    const lines = data.toString().split('\n');
    lines.forEach(line => {
      if (line.trim()) console.log(`${prefix}${line}`);
    });
  };

  child.stdout.on('data', log);
  child.stderr.on('data', log);

  child.on('close', (code) => {
    console.log(`${prefix}Process exited with code ${code}`);
  });

  children.push(child);
});

// Graceful shutdown: When you hit Ctrl+C, kill all child processes.
process.on('SIGINT', () => {
  console.log('\n🛑 Shutting down cluster...');
  children.forEach(child => child.kill());
  process.exit();
});
