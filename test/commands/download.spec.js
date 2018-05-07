'use strict';
const assert = require('assert');
const fse = require('fs-extra');
const spawn = require('child_process').spawn;
const path = require('path');

const download = require('../../lib/commands/download').api;

const TMP_PATH = path.join(__dirname, '..', 'tmp');
const OUT_PATH = path.join(TMP_PATH, 'download');
const REPO_URL = 'https://github.com/lacodda/test.git';

describe('download repo when it exists', () => {

  beforeEach(() => {
    return fse.remove(OUT_PATH).catch(err => {
      console.log(err);

      throw err;
    });
  });

  describe('cli', () => {
    it('can download', (done) => {
      let buffer = '';

      const child = spawn('node', [
        './bin/lyrn', 'download', REPO_URL, OUT_PATH,
      ], { cwd: __dirname + '/../..' });

      child.stdout.on('data', (b) => {
        buffer += b.toString();
      });

      child.on('close', () => {
        assert.equal(
          `Repository ${REPO_URL} successfully downloaded\n`,
          buffer,
        );
        done();
      });
    });
  });

  describe('api', () => {
    it('can download', () => {
      return download(REPO_URL, OUT_PATH).then(results => {
        for (let entry in results) {
          assert.equal(REPO_URL, entry);
        }
      });
    });
  });
});
