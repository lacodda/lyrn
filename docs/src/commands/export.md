# lyrn export

Export configuration files:

```bash
lyrn export config
```

This command exports Webpack configurations for development (`webpack.config.dev.js`) and production (`webpack.config.prod.js`) environments.

Also, as a result of the export, an entry will be made in the `lyrn.json` file about the exported files:

```json
{
  "dev": {
    "config": "webpack.config.dev.js"
  },
  "prod": {
    "config": "webpack.config.prod.js"
  }
}
```

<div class="warning">

Please note that specifying configuration files in `lyrn.json` leads to their import and merging with the default Webpack configuration when using the [start][start] and [build][build] commands next time.

</div>

[start]: ./start.html
[build]: ./build.html