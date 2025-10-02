const fs = require('fs');
const path = require('path');

function cleanup() {
  const binDir = path.join(__dirname, '..', 'bin');
  
  if (fs.existsSync(binDir)) {
    try {
      const files = fs.readdirSync(binDir);
      files.forEach(file => {
        const filePath = path.join(binDir, file);
        fs.unlinkSync(filePath);
        console.log(`🗑️  Removed: ${filePath}`);
      });
      fs.rmdirSync(binDir);
      console.log('✅ Aether CLI uninstalled successfully!');
    } catch (error) {
      console.error('⚠️  Error during cleanup:', error.message);
    }
  }
}

if (require.main === module) {
  cleanup();
}

module.exports = { cleanup };
