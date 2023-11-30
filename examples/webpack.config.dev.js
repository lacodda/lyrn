const path = require('path');
const webpack = require('webpack');
const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const cwd = process.cwd();
const isDev = true;

module.exports = {
  cache: false,
  devServer: {
    compress: true,
    headers: {
      'Access-Control-Allow-Headers': 'X-Requested-With, content-type, Authorization',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
      'Access-Control-Allow-Origin': '*'
    },
    historyApiFallback: true,
    hot: true,
    port: 8080,
    static: './'
  },
  devtool: 'inline-source-map',
  entry: [
    path.resolve(cwd, 'src/main.ts')
  ],
  infrastructureLogging: {
    level: 'warn'
  },
  mode: 'development',
  module: {
    rules: [
      new Object({
        test: /\.tsx?$/,
        exclude: /(node_modules|\.webpack)/,
        use: {
          loader: 'ts-loader',
          options: {
            transpileOnly: true,
          },
        },
      }),
      new Object({
        test: /\.(sass|scss|css)$/,
        use: [
          { loader: isDev ? 'style-loader' : MiniCssExtractPlugin.loader },
          {
            loader: 'css-loader',
            options: {
              importLoaders: isDev ? 1 : 2,
              sourceMap: isDev,
              modules: isDev,
            },
          },
          { loader: 'postcss-loader', options: { sourceMap: isDev } },
          { loader: 'sass-loader', options: { sourceMap: isDev } },
        ],
      }),
      new Object({
        test: /\.(?:ico|gif|png|jpe?g)$/i,
        type: 'asset/resource',
        generator: {
          filename: 'assets/[hash][ext][query]',
        },
      }),
      new Object({
        test: /\.(woff(2)?|eot|ttf|otf|svg|)$/i,
        type: 'asset/inline',
      })
    ],
  },
  optimization: {
    minimize: false
  },
  output: {
    assetModuleFilename: 'assets/[hash][ext][query]',
    filename: 'js/[name].[contenthash].bundle.js',
    path: path.resolve(cwd, 'dist'),
    publicPath: 'http://localhost:8080/'
  },
  performance: {
    hints: false
  },
  plugins: [
    new ForkTsCheckerWebpackPlugin(),
    new CopyWebpackPlugin({
      patterns: [{
        from: path.resolve(cwd, 'public'),
        to: 'assets',
        globOptions: {
          ignore: ['*.DS_Store'],
        },
        noErrorOnMissing: true, 
      }],
    }),
    new HtmlWebpackPlugin({
      title: 'APP TITLE',
      favicon: path.resolve(cwd, './src/images/logo.svg'),
      template: path.resolve(cwd, './src/index.html'),
      filename: 'index.html'
    }),
    new webpack.HotModuleReplacementPlugin()
  ],
  resolve: {
    alias: {
      build: path.resolve(cwd, 'dist'),
      images: path.resolve(cwd, 'src/images'),
      main: path.resolve(cwd, 'src/main.ts'),
      public: path.resolve(cwd, 'public'),
      src: path.resolve(cwd, 'src')
    },
    extensions: [
      '.tsx',
      '.ts',
      '.mjs',
      '.js',
      '.jsx',
      '.json',
      '.wasm',
      '.css'
    ],
    modules: [
      path.resolve(cwd, 'src'),
      'node_modules'
    ]
  },
  stats: {
    assets: false,
    modules: false
  },
  target: 'web'
}
