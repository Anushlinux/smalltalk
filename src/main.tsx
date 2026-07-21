import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AuthProvider, useAuth } from "./auth/AuthProvider";
import { AuthLoadingScreen, AuthScreen } from "./auth/AuthScreen";

function AuthenticatedApp() {
  const { initialized, session } = useAuth();

  if (!initialized) return <AuthLoadingScreen />;
  if (!session) return <AuthScreen />;
  return <App />;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AuthProvider>
      <AuthenticatedApp />
    </AuthProvider>
  </React.StrictMode>,
);
