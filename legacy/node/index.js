// Loads the addon of this platform. The addon holds every rule; this file only finds it.
'use strict';

const { platform, arch } = process;
const libc = platform === 'linux' ? (process.report?.getReport().header.glibcVersionRuntime ? '-gnu' : '-musl') : '';
module.exports = require(`./lib/http-forms.${platform}-${arch}${libc}.node`);
