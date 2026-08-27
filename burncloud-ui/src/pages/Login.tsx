import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'motion/react';
import { 
  ShieldCheck, 
  ArrowRight, 
  Lock, 
  Mail, 
  Key, 
  CheckCircle2, 
  Fingerprint, 
  AlertCircle,
  Building2,
  Sparkles
} from 'lucide-react';
import { Button, Card, Input, Badge } from '@/components/ui';
import { Logo } from '@/components/Logo';
import { useTranslation } from '@/i18n/I18nContext';
import { LanguageSwitcher } from '@/components/LanguageSwitcher';

export function Login() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const [email, setEmail] = useState('wei@burncloud.io');
  const [password, setPassword] = useState('••••••••••••');
  const [rememberMe, setRememberMe] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [authMethod, setAuthMethod] = useState<'password' | 'passkey'>('password');
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setStatusMessage(t.publicPages.login.authenticating);

    setTimeout(() => {
      setIsLoading(false);
      navigate('/');
    }, 600);
  };

  const handlePasskeyLogin = () => {
    setIsLoading(true);
    setStatusMessage(t.publicPages.login.probingHardware);

    setTimeout(() => {
      setIsLoading(false);
      navigate('/');
    }, 800);
  };

  return (
    <div className="min-h-screen bg-[#F9FAFB] text-gray-900 font-sans flex flex-col justify-between selection:bg-gray-900 selection:text-white">
      {/* Top Header */}
      <header className="p-6 flex items-center justify-between max-w-7xl w-full mx-auto">
        <Link to="/home" className="flex items-center gap-2.5 group">
          <Logo className="w-7 h-7 group-hover:scale-105 transition-transform" />
          <span className="font-display font-bold text-lg tracking-tight text-gray-900">BurnCloud</span>
        </Link>
        <div className="flex items-center gap-4 text-xs text-gray-500">
          <LanguageSwitcher variant="compact" />
          <span>{t.publicPages.login.dontHaveAccount}</span>
          <Link to="/register" className="font-semibold text-gray-900 hover:underline">
            {t.publicPages.login.createAccount}
          </Link>
        </div>
      </header>

      {/* Main Login Card */}
      <main className="flex-1 flex items-center justify-center p-6 my-auto">
        <div className="w-full max-w-md space-y-6">
          <motion.div 
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            className="text-center space-y-2"
          >
            <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-blue-50 border border-blue-100 text-blue-700 text-[11px] font-medium font-mono mb-1">
              <ShieldCheck className="w-3.5 h-3.5 text-blue-600" />
              <span>{t.publicPages.login.hardwarePortal}</span>
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-gray-950">
              {t.publicPages.login.title}
            </h1>
            <p className="text-xs text-gray-500">
              {t.publicPages.login.subtitle}
            </p>
          </motion.div>

          <Card className="p-8 space-y-6 shadow-xl border-gray-200/80 bg-white">
            {/* Auth Method Switcher */}
            <div className="flex rounded-xl bg-gray-100 p-1 text-xs font-medium">
              <button
                type="button"
                onClick={() => setAuthMethod('password')}
                className={`flex-1 py-1.5 rounded-lg transition-all ${
                  authMethod === 'password' ? 'bg-white text-gray-950 shadow-sm font-semibold' : 'text-gray-500 hover:text-gray-900'
                }`}
              >
                {t.publicPages.login.passwordAnd2fa}
              </button>
              <button
                type="button"
                onClick={() => setAuthMethod('passkey')}
                className={`flex-1 py-1.5 rounded-lg transition-all flex items-center justify-center gap-1.5 ${
                  authMethod === 'passkey' ? 'bg-white text-gray-950 shadow-sm font-semibold' : 'text-gray-500 hover:text-gray-900'
                }`}
              >
                <Fingerprint className="w-3.5 h-3.5 text-indigo-600" />
                <span>{t.publicPages.login.passkeyEnclave}</span>
              </button>
            </div>

            {authMethod === 'password' ? (
              <form onSubmit={handleSubmit} className="space-y-4">
                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-gray-700 block">
                    {t.publicPages.login.workEmail}
                  </label>
                  <div className="relative">
                    <Mail className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                    <Input
                      type="email"
                      required
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      placeholder="name@company.com"
                      className="pl-9 text-xs"
                    />
                  </div>
                </div>

                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs">
                    <label className="font-semibold text-gray-700">{t.publicPages.login.password}</label>
                    <a href="#forgot" onClick={(e) => { e.preventDefault(); alert("A password reset token has been issued to your registered hardware key."); }} className="text-blue-600 hover:underline font-medium">
                      {t.publicPages.login.forgot}
                    </a>
                  </div>
                  <div className="relative">
                    <Lock className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                    <Input
                      type="password"
                      required
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder="••••••••••••"
                      className="pl-9 text-xs"
                    />
                  </div>
                </div>

                <div className="flex items-center justify-between text-xs">
                  <label className="flex items-center gap-2 cursor-pointer text-gray-600 select-none">
                    <input
                      type="checkbox"
                      checked={rememberMe}
                      onChange={(e) => setRememberMe(e.target.checked)}
                      className="rounded border-gray-300 text-gray-900 focus:ring-gray-900"
                    />
                    <span>{t.publicPages.login.rememberSession}</span>
                  </label>
                  <span className="text-[10px] text-gray-400 font-mono">256-bit TLS</span>
                </div>

                {statusMessage && (
                  <div className="p-3 bg-blue-50 border border-blue-100 rounded-xl text-xs text-blue-800 flex items-center gap-2 font-mono">
                    <div className="w-3.5 h-3.5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin shrink-0" />
                    <span>{statusMessage}</span>
                  </div>
                )}

                <Button
                  type="submit"
                  disabled={isLoading}
                  className="w-full h-10 gap-2 text-xs font-semibold"
                >
                  {isLoading ? t.common.loading : t.publicPages.login.signInBtn}
                  {!isLoading && <ArrowRight className="w-4 h-4" />}
                </Button>
              </form>
            ) : (
              <div className="space-y-4 py-2 text-center">
                <div className="w-16 h-16 rounded-2xl bg-indigo-50 border border-indigo-100 flex items-center justify-center mx-auto text-indigo-600">
                  <Fingerprint className="w-8 h-8" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-gray-900">{t.publicPages.login.tpmDeviceDetected}</h3>
                  <p className="text-xs text-gray-500 mt-1 max-w-xs mx-auto">
                    Authenticate using your physical hardware key or biometric enclave bound to your account.
                  </p>
                </div>

                {statusMessage && (
                  <div className="p-3 bg-indigo-50 border border-indigo-100 rounded-xl text-xs text-indigo-800 flex items-center justify-center gap-2 font-mono">
                    <div className="w-3.5 h-3.5 border-2 border-indigo-600 border-t-transparent rounded-full animate-spin shrink-0" />
                    <span>{statusMessage}</span>
                  </div>
                )}

                <Button
                  onClick={handlePasskeyLogin}
                  disabled={isLoading}
                  className="w-full h-10 gap-2 text-xs font-semibold bg-gray-900"
                >
                  <Fingerprint className="w-4 h-4 text-indigo-400" />
                  <span>{t.publicPages.login.authenticatePasskey}</span>
                </Button>
              </div>
            )}

            {/* Quick Demo Login Option */}
            <div className="pt-4 border-t border-gray-100 text-center space-y-3">
              <span className="text-[11px] text-gray-400 font-medium block uppercase tracking-wider">
                {t.publicPages.login.orQuickDrive}
              </span>
              <Button
                variant="secondary"
                onClick={() => {
                  setIsLoading(true);
                  setTimeout(() => navigate('/'), 400);
                }}
                className="w-full h-9 text-xs gap-1.5 text-gray-700"
              >
                <Sparkles className="w-3.5 h-3.5 text-amber-500" />
                <span>{t.publicPages.login.instantDemoLogin}</span>
              </Button>
            </div>
          </Card>

          <p className="text-center text-[11px] text-gray-400 font-mono">
            BurnCloud Security Enclave • Hardware Proof Protocol v2.4
          </p>
        </div>
      </main>

      {/* Footer */}
      <footer className="p-6 text-center text-xs text-gray-400">
        <div className="flex justify-center items-center gap-4">
          <Link to="/home" className="hover:text-gray-600">Home</Link>
          <span>•</span>
          <Link to="/register" className="hover:text-gray-600">Register</Link>
          <span>•</span>
          <a href="#privacy" className="hover:text-gray-600">Privacy Policy</a>
          <span>•</span>
          <a href="#terms" className="hover:text-gray-600">Terms of Service</a>
        </div>
      </footer>
    </div>
  );
}
