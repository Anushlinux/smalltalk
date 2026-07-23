import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AuthProvider, useAuth } from "./auth/AuthProvider";
import { AuthLoadingScreen, AuthScreen } from "./auth/AuthScreen";
import { PermissionsGate } from "./permissions/PermissionsScreen";
import { AppUpdatePrompt } from "./updates/AppUpdatePrompt";
import { AppUpdateProvider } from "./updates/AppUpdateProvider";

function AuthenticatedApp() {
  const { initialized, session, user, profile, signOut } = useAuth();

  if (!initialized) return <AuthLoadingScreen />;
  if (!session) return <AuthScreen />;
  return (
    <PermissionsGate
      accountEmail={user?.email || profile?.email || "Signed in"}
      onSignOut={() => void signOut()}
    >
      <App />
    </PermissionsGate>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AuthProvider>
      <AppUpdateProvider>
        <AuthenticatedApp />
        <AppUpdatePrompt />
      </AppUpdateProvider>
    </AuthProvider>
  </React.StrictMode>,
);
