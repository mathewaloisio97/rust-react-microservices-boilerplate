const { spawn } = require('child_process');

const RESET = '\x1b[0m';
const COLORS = {
  docker: '\x1b[36m', // Cyan (Backend Docker Stack)
  react: '\x1b[31m',  // Red  (React Frontend)
};

console.log('🚀 Booting YourApp Local Development Cluster (Docker + Vite)...\n');

// Check if the user explicitly wants to force a Docker image build
const forceBuild = process.argv.includes('--build');

// On Windows, use cmd.exe /c to run binaries/batch files cleanly without trigger-warning flags.
const isWin = process.platform === 'win32';

const runCmd = (cmd, args, cwd = './') => {
  if (isWin) {
    return spawn('cmd.exe', ['/c', cmd, ...args], {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe']
    });
  }
  return spawn(cmd, args, {
    cwd,
    stdio: ['ignore', 'pipe', 'pipe']
  });
};

// 1. Start Docker Compose for the entire backend cluster.
// If --build argument is present, force build; otherwise, allow default fast startup.
const dockerArgs = forceBuild ? ['compose', 'up', '--build'] : ['compose', 'up'];
const docker = runCmd('docker', dockerArgs);

// 2. Start React development server.
const react = runCmd('pnpm', ['dev', '--mode', 'development'], './website');

const attachLogger = (child, prefix) => {
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
};

attachLogger(docker, `${COLORS.docker}[Docker Backend]${RESET} | `);
attachLogger(react, `${COLORS.react}[React Frontend]${RESET} | `);

// Graceful shutdown: When hitting Ctrl+C, shut down React and tear down Docker containers cleanly.
process.on('SIGINT', () => {
  console.log('\n🛑 Shutting down cluster...');
  react.kill();

  const dockerDown = runCmd('docker', ['compose', 'down']);
  dockerDown.on('close', () => {
    process.exit();
  });
});
