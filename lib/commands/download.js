'use strict';

const spawn = require('child_process').spawn;
const fse = require('fs-extra');
const clone = require('../util/clone').api;

function download(repo, dest, opts, fn) {
  return new Promise((resolve, reject) => {
    if (typeof opts === 'function') {
      fn = opts;
      opts = null;
    }
    opts = opts || {};

    // repo = normalize(repo);
    // const url = getUrl(repo, clone);
    const url = repo;

    clone(url, dest, { checkout: repo.checkout, shallow: repo.checkout === 'master' }).then(results => {
      Object.keys(results).forEach(entry => {
        // let msg = `Repository ${entry} could not be cloned`;
        if (results[entry]) {
          fse.remove(dest + '/.git');
          // msg = `Repository ${entry} successfully cloned`;
        }
        // console.log(msg);
        return resolve(results);
      });
    }).catch(err => {
      return reject(err);
    });
  });
}

exports.api = download;

function cli(url, name, cloneOpts = {}) {
  return new Promise((resolve, reject) => {
    download(url, name, cloneOpts).then((results) => {
      // print on stdout for terminal users
      Object.keys(results).forEach((entry) => {
        let msg = `Repository ${entry} could not be downloaded`;
        if (results[entry]) {
          msg = `Repository ${entry} successfully downloaded`;
        }
        console.log(msg);
        resolve(results);
      });
    }).catch(err => {
      reject(err);
    });
  });
}

exports.cli = cli;

function normalize(repo) {
  const regex = /^((github|gitlab|bitbucket):)?((.+):)?([^/]+)\/([^#]+)(#(.+))?$/;
  const match = regex.exec(repo);
  const type = match[2] || 'github';
  let origin = match[4] || null;
  const owner = match[5];
  const name = match[6];
  const checkout = match[8] || 'master';

  if (origin == null) {
    if (type === 'github')
      origin = 'github.com';
    else if (type === 'gitlab')
      origin = 'gitlab.com';
    else if (type === 'bitbucket')
      origin = 'bitbucket.org';
  }

  return {
    type: type,
    origin: origin,
    owner: owner,
    name: name,
    checkout: checkout,
  };
}



function addProtocol(origin, clone) {
  if (!/^(f|ht)tps?:\/\//i.test(origin)) {
    if (clone)
      origin = 'git@' + origin;
    else
      origin = 'https://' + origin;
  }

  return origin;
}



function getUrl(repo, clone) {
  let url;

  // Get origin with protocol and add trailing slash or colon (for ssh)
  let origin = addProtocol(repo.origin, clone);
  if (/^git\@/i.test(origin))
    origin = origin + ':';
  else
    origin = origin + '/';

  // Build url
  if (clone) {
    url = origin + repo.owner + '/' + repo.name + '.git';
  } else {
    if (repo.type === 'github')
      url = origin + repo.owner + '/' + repo.name + '/archive/' + repo.checkout + '.zip';
    else if (repo.type === 'gitlab')
      url = origin + repo.owner + '/' + repo.name + '/repository/archive.zip?ref=' + repo.checkout;
    else if (repo.type === 'bitbucket')
      url = origin + repo.owner + '/' + repo.name + '/get/' + repo.checkout + '.zip';
    else
      url = github(repo);
  }

  return url;
}
