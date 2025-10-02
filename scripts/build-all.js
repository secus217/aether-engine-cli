const { exec } = require('child_process');
const fs = require('fs');
const path = require('path');

const TARGETS = [
  { name: 'linux-x64', target: 'x86_64-unknown-linux-gnu' },
  { name: 'linux-arm64', target: 'aarch64-unknown-linux-gnu' },
  { name: 'macos-x64', target: 'x86_64-apple-darwin' },
  { name: 'macos-arm64', target: 'aarch64-apple-darwin' },
  { name: 'windows-x64', target: 'x86_64-pc-windows-gnu' },
];

function runCommand(command) {
  return new Promise((resolve, reject) => {
    console.log(`🔨 Running: ${command}`);
    exec(command, (error, stdout, stderr) => {
      if (error) {
        reject(new Error(`${error.message}\n${stderr}`));
      } else {
        console.log(stdout);
        resolve(stdout);
      }
    });
  });
}

async function buildAll() {
  const binariesDir = path.join(__dirname, '..', 'binaries');
  
  // Create binaries directory
  if (!fs.existsSync(binariesDir)) {
    fs.mkdirSync(binariesDir, { recursive: true });
  }
  
  console.log('🚀 Building Aether CLI for all platforms...\n');
  
  for (const { name, target } of TARGETS) {
    try {
      console.log(`📦 Building for ${name} (${target})...`);
      
      // Add target if not installed
      await runCommand(`rustup target add ${target}`);
      
      // Build for target
      await runCommand(`cargo build --release --target ${target}`);
      
      // Copy binary to binaries directory
      const extension = name.includes('windows') ? '.exe' : '';
      const sourcePath = path.join('target', target, 'release', `aether${extension}`);
      const destPath = path.join(binariesDir, `aether-${name}${extension}`);
      
      if (fs.existsSync(sourcePath)) {
        fs.copyFileSync(sourcePath, destPath);
        console.log(`✅ Built and copied: ${destPath}\n`);
      } else {
        console.log(`⚠️  Binary not found: ${sourcePath}\n`);
      }
      
    } catch (error) {
      console.error(`❌ Failed to build for ${name}:`, error.message);
    }
  }
  
  console.log('🎉 Build process completed!');
  console.log('📁 Binaries available in:', binariesDir);
}

if (require.main === module) {
  buildAll().catch(console.error);
}

module.exports = { buildAll };
