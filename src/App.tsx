import { useEffect, useState } from "react";

import { Notch } from "./components/Notch";
import { Settings } from "./pages/Settings";

/**
 * Hash-based "routing". The notch window loads the bare app and the
 * settings Tauri window navigates to `index.html#/settings`; we don't
 * need a router dependency for two routes.
 */
function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const onChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return hash;
}

function App() {
  const hash = useHashRoute();
  if (hash === "#/settings") {
    return <Settings />;
  }
  return <Notch />;
}

export default App;
