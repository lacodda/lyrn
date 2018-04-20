'use strict';
const assert = require('assert');
const fs = require('fs-extra');
const spawn = require('child_process').spawn;
const git = require('nodegit');
const Repository = git.Repository;
const path = require('path');
const local = path.join.bind(path, __dirname);

const clone = require('../lib/clone').api;

describe('clone repo when it exists', () => {

  const clonePath = local('./repos/clone');

  beforeEach(() => {
    return fs.remove(clonePath).catch(err => {
      console.log(err);

      throw err;
    });
  });

  describe('cli', (done) => {
    it('can clone', (done) => {
      const url = 'https://github.com/lacodda/lyrn.git';
      let buffer = '';

      const child = spawn('node', [
        './bin/lyrn', 'clone', url, clonePath,
      ], { cwd: __dirname + '/..' });

      child.stdout.on('data', (b) => {
        buffer += b.toString();
      });

      child.on('close', () => {
        assert.equal(
          'Repository https://github.com/lacodda/lyrn.git successfully cloned\n',
          buffer,
        );
        done();
      });
    });
  });

  describe('api', (done) => {
    it('can clone', () => {
      const test = this;
      const url = 'https://github.com/lacodda/lyrn.git';
      const opts = {
        fetchOpts: {
          callbacks: {
            certificateCheck: function () {
              return 1;
            },
          },
        },
      };

      return clone(url, clonePath, opts).then(results => {
        for (let entry in results) {
          assert.ok(results[entry] instanceof Repository);
        }
      });
    });
  });
});
