const fs = require('fs');
const path = require('path');
const https = require('https');
const { exec } = require('child_process');

const GITHUB_REPO = 'secus217/aether-cli';
const BINARY_NAME = 'aether';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;
  
  if (platform === 'win32') {
    return arch === 'x64' ? 'windows-x64' : 'windows-arm64';
  } else if (platform === 'darwin') {
    return arch === 'x64' ? 'macos-x64' : 'macos-arm64';
  } else if (platform === 'linux') {
    return arch === 'x64' ? 'linux-x64' : 'linux-arm64';
  }
  
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function downloadBinary() {
  return new Promise((resolve, reject) => {
    const platformSuffix = getPlatform();
    const binaryName = process.platform === 'win32' ? `${BINARY_NAME}.exe` : BINARY_NAME;
    const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/latest/download/aether-${platformSuffix}${process.platform === 'win32' ? '.exe' : ''}`;
    
    console.log(`🚀 Downloading Aether CLI binary for ${platformSuffix}...`);
    console.log(`📥 URL: ${downloadUrl}`);
    
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    const binaryPath = path.join(binDir, binaryName);
    const file = fs.createWriteStream(binaryPath);
    
    https.get(downloadUrl, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          if (redirectResponse.statusCode === 200) {
            redirectResponse.pipe(file);
            file.on('finish', () => {
              file.close();
              // Make binary executable on Unix systems
              if (process.platform !== 'win32') {
                fs.chmodSync(binaryPath, '755');
              }
              console.log(`✅ Aether CLI installed successfully!`);
              console.log(`🎯 Binary location: ${binaryPath}`);
              resolve();
            });
          } else {
            reject(new Error(`Failed to download binary: ${redirectResponse.statusCode}`));
          }
        }).on('error', reject);
      } else if (response.statusCode === 200) {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          // Make binary executable on Unix systems
          if (process.platform !== 'win32') {
            fs.chmodSync(binaryPath, '755');
          }
          console.log(`✅ Aether CLI installed successfully!`);
          console.log(`🎯 Binary location: ${binaryPath}`);
          resolve();
        });
      } else {
        reject(new Error(`Failed to download binary: ${response.statusCode}`));
      }
    }).on('error', reject);
    
    file.on('error', (err) => {
      fs.unlink(binaryPath, () => {}); // Delete the file on error
      reject(err);
    });
  });
}

// Fallback: create a JS wrapper if binary download fails
function createJsWrapper() {
  const binDir = path.join(__dirname, '..', 'bin');
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  const wrapperPath = path.join(binDir, 'aether.js');
  const wrapperContent = `#!/usr/bin/env node
console.error('❌ Aether CLI binary not available for your platform.');
console.error('📖 Please visit: https://github.com/${GITHUB_REPO}/releases');
console.error('🔽 Download the appropriate binary for your system.');
process.exit(1);
`;
  
  fs.writeFileSync(wrapperPath, wrapperContent);
  fs.chmodSync(wrapperPath, '755');
  console.log('⚠️  Created fallback wrapper. Please install binary manually.');
}

async function install() {
  try {
    // First try to use the binary included in the package
    const sourceBinaryPath = path.join(__dirname, '..', 'bin', 'aether');
    if (fs.existsSync(sourceBinaryPath) && fs.statSync(sourceBinaryPath).size > 0) {
      console.log('✅ Using pre-included binary');
      // Make sure it's executable
      if (process.platform !== 'win32') {
        fs.chmodSync(sourceBinaryPath, '755');
      }
      return;
    }
    
    // Fallback to download if no binary in package
    await downloadBinary();
  } catch (error) {
    console.error('⚠️  Failed to setup binary:', error.message);
    console.log('📦 Creating fallback wrapper...');
    createJsWrapper();
  }
}

if (require.main === module) {
  install().catch(console.error);
}

module.exports = { install, getPlatform };
