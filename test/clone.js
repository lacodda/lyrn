'use strict';
const assert = require('assert');
const fs = require('fs-extra');
const spawn = require('child_process').spawn;
const path = require('path');

const clone = require('../lib/clone').api;

const TMP_PATH = path.join(__dirname, '..', 'tmp');
const OUT_PATH = path.join(TMP_PATH, 'clone');
const REPO_URL = 'https://github.com/lacodda/test.git';

describe('clone repo when it exists', () => {

  beforeEach(() => {
    return fs.remove(TMP_PATH).catch(err => {
      console.log(err);

      throw err;
    });
  });

  describe('cli', (done) => {
    it('can clone', (done) => {
      let buffer = '';

      const child = spawn('node', [
        './bin/lyrn', 'clone', REPO_URL, OUT_PATH,
      ], { cwd: __dirname + '/..' });

      child.stdout.on('data', (b) => {
        buffer += b.toString();
      });

      child.on('close', () => {
        assert.equal(
          `Repository ${REPO_URL} successfully cloned\n`,
          buffer,
        );
        done();
      });
    });
  });

  describe('api', (done) => {
    it('can clone', () => {
      return clone(REPO_URL, OUT_PATH).then(results => {
        for (let entry in results) {
          assert.equal(REPO_URL, entry);
        }
      });
    });
  });
});
