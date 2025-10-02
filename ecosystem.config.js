module.exports = {
  apps: [{
    name: 'kizo-backend',
    script: './target/release/kizo-server',
    instances: 1,
    autorestart: true,
    watch: false,
    max_memory_restart: '1G',
    env: {
      NODE_ENV: 'production'
    }
  }]
}
