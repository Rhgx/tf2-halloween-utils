import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { openLink } from './api.ts';
import { App } from './App.tsx';
// All CSS is imported here, in one fixed order, because later files win ties: base first, then the shell,
// then the tabs and shared parts in the order they were split from one file.
import './styles/base.css';
import './styles/shell.css';
import './features/contracts/contracts.css';
import './styles/common.css';
import './components/fields.css';
import './features/settings/settings.css';
import './features/overlay/overlay.css';

// The app window can't open new browser tabs, so every link to an outside page opens in the default browser.
document.addEventListener('click', (event) => {
  const link = event.target instanceof Element ? event.target.closest('a[href^="https://"]') : null;
  if (!(link instanceof HTMLAnchorElement)) return;
  event.preventDefault();
  openLink(link.href).catch((err: unknown) => console.error(err));
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
