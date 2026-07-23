import smalltalkLogo from "../assets/smalltalk-logo.png";
import { useAuth } from "./AuthProvider";
import "./AuthScreen.css";

function GoogleIcon() {
  return (
    <svg className="auth-google-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path fill="#4285F4" d="M21.6 12.227c0-.709-.064-1.391-.182-2.045H12v3.868h5.382a4.6 4.6 0 0 1-1.996 3.018v2.509h3.232c1.891-1.741 2.982-4.305 2.982-7.35Z" />
      <path fill="#34A853" d="M12 22c2.7 0 4.964-.895 6.618-2.423l-3.232-2.509c-.895.6-2.041.955-3.386.955-2.605 0-4.809-1.759-5.596-4.123H3.064v2.591A9.997 9.997 0 0 0 12 22Z" />
      <path fill="#FBBC05" d="M6.404 13.9A6.01 6.01 0 0 1 6.091 12c0-.659.114-1.3.313-1.9V7.509h-3.34A9.997 9.997 0 0 0 2 12c0 1.614.386 3.141 1.064 4.491L6.404 13.9Z" />
      <path fill="#EA4335" d="M12 5.977c1.468 0 2.786.505 3.827 1.496l2.864-2.864C16.959 2.995 14.695 2 12 2a9.997 9.997 0 0 0-8.936 5.509l3.34 2.591C7.191 7.736 9.395 5.977 12 5.977Z" />
    </svg>
  );
}

export function AuthScreen() {
  const { loading, error, signInWithGoogle } = useAuth();

  return (
    <main className="auth-screen">
      <section className="auth-card" aria-labelledby="auth-title">
        <header className="auth-heading">
          <h1 id="auth-title">Sign in to Smalltalk</h1>
        </header>

        <button
          className="auth-google-button"
          type="button"
          disabled={loading}
          aria-busy={loading}
          onClick={() => void signInWithGoogle()}
        >
          <GoogleIcon />
          <span>{loading ? "Opening Google…" : "Continue with Google"}</span>
        </button>

        {error ? <p className="auth-error" role="alert">{error}</p> : null}

        <p className="auth-divider">OR</p>

        <input
          className="auth-email-input"
          type="email"
          aria-label="Email"
          placeholder="Enter your email"
          readOnly
        />

        <button className="auth-email-button" type="button" disabled>
          Continue with email
        </button>

        <p className="auth-privacy-copy">
          By continuing, you acknowledge smalltalk's <span>Privacy Policy.</span>
        </p>
      </section>
    </main>
  );
}

export function AuthLoadingScreen() {
  return (
    <main className="auth-screen auth-loading-screen" aria-label="Opening Smalltalk">
      <img src={smalltalkLogo} alt="" />
      <p>Opening Smalltalk…</p>
    </main>
  );
}
