import fs from 'node:fs';
import {spawn} from 'node:child_process';
const [file,...args]=process.argv.slice(2);
if(!file||!args.length)throw new Error('Usage: node run.mjs /absolute/scratch/env.json test [Playwright arguments]');
const env={...process.env,...JSON.parse(fs.readFileSync(file,'utf8'))};
const child=spawn(process.execPath,['node_modules/@playwright/test/cli.js',...args],{env,stdio:'inherit'});
child.on('exit',code=>process.exit(code??1));
