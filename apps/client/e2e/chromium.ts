import { existsSync } from 'node:fs';
import { chromium } from 'playwright-core';

process.exit(existsSync(chromium.executablePath()) ? 0 : 1);
