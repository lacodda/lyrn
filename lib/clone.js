'use strict';

const git = require('nodegit');
const path = require('path');

function clone(url, name, cloneOpts = {}) {
  return new Promise((resolve, reject) => {
    git.Clone(url, name, cloneOpts).then(repo => {
      return resolve({ [url]: repo });
    }).catch((err) => {
      return reject(err);
    });
  });
}

exports.api = clone;

function cli(url, name, cloneOpts = {}) {
  return new Promise((resolve, reject) => {
    clone(url, name, cloneOpts).then((results) => {
      // print on stdout for terminal users
      Object.keys(results).forEach((entry) => {
        let msg = `Repository ${entry} could not be cloned`;
        if (results[entry]) {
          msg = `Repository ${entry} successfully cloned`;
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
