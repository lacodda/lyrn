const { getRepoUrl } = require('../../lib/util/helpers');

describe('test helpers functions', () => {
  describe('test getRepoUrl function', () => {
    it('full url worked', () => {
      const result = getRepoUrl('https://test-repo.com/test/test.git');
      expect(result).to.equal('https://test-repo.com/test/test.git');
    });

    it('github owner/repo url', () => {
      const result = getRepoUrl('user/repo');
      expect(result).to.equal('https://github.com/user/repo.git');
    });

    it('github lacodda/repo url', () => {
      const result = getRepoUrl('repo');
      expect(result).to.equal('https://github.com/lacodda/repo.git');
    });
  });
});
