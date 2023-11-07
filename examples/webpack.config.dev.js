const path = require('path');
const webpack = require('webpack');
const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');
const cwd = process.cwd();
const isDev = true;

module.exports = {
  cache: false,
  devServer: {
    compress: true,
    headers: {
      "Access-Control-Allow-Headers": "X-Requested-With, content-type, Authorization",
      "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, PATCH, OPTIONS",
      "Access-Control-Allow-Origin": "*"
    },
    historyApiFallback: true,
    hot: true,
    port: 8080,
    static: "./"
  },
  devtool: "inline-source-map",
  entry: [
    "C:\\Projects\\lyrn\\src\\main.ts"
  ],
  infrastructureLogging: {
    level: "warn"
  },
  mode: "development",
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
    assetModuleFilename: "assets/[hash][ext][query]",
    filename: "js/[name].[contenthash].bundle.js",
    path: "C:\\Projects\\lyrn\\dist",
    publicPath: "http://localhost:8080/"
  },
  performance: {
    hints: false
  },
  plugins: [
    new ForkTsCheckerWebpackPlugin(),
    new CopyWebpackPlugin({
      patterns: [{
        from: 'C:\Projects\lyrn\public',
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
    new webpack.HotModuleReplacementPlugin(),
    new ReactRefreshWebpackPlugin()
  ],
  resolve: {
    alias: {
      build: "C:\\Projects\\lyrn\\dist",
      images: "C:\\Projects\\lyrn\\src\\images",
      main: "C:\\Projects\\lyrn\\src\\main.ts",
      public: "C:\\Projects\\lyrn\\public",
      src: "C:\\Projects\\lyrn\\src"
    },
    extensions: [
      ".tsx",
      ".ts",
      ".mjs",
      ".js",
      ".jsx",
      ".json",
      ".wasm",
      ".css"
    ],
    modules: [
      "C:\\Projects\\lyrn\\src",
      "node_modules"
    ]
  },
  stats: {
    assets: false,
    modules: false
  },
  target: "web"
}
