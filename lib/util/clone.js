'use strict';

const spawn = require('child_process').spawn;

const clone = async (repo, targetPath, opts, cb) => {
  return new Promise((resolve, reject) => {
    if (typeof opts === 'function') {
      cb = opts;
      opts = null;
    }

    opts = opts || {};

    const git = opts.git || 'git';
    const args = ['clone'];

    if (opts.shallow) {
      args.push('--depth');
      args.push('1');
    }

    args.push('--');
    args.push(repo);
    args.push(targetPath);

    const process = spawn(git, args);
    let output = {};
    process.on('close', status => {
      if (status == 0) {
        if (opts.checkout) {
          _checkout();
        } else {
          output[repo] = 'cloned';
          return resolve(output);
          // cb && cb();
        }
      } else {
        return reject(new Error('\'git clone\' failed with status ' + status));
        // cb && cb(new Error('\'git clone\' failed with status ' + status));
      }
    });

    function _checkout() {
      const args = ['checkout', opts.checkout];
      const process = spawn(git, args, { cwd: targetPath });
      process.on('close', function (status) {
        if (status == 0) {
          output[repo] = 'cloned';
          return resolve(output);
          // cb && cb();
        } else {
          return reject(new Error('\'git clone\' failed with status ' + status));
          // cb && cb(new Error('\'git checkout\' failed with status ' + status));
        }
      });
    }
  });
};

module.exports = clone;
