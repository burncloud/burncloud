import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'motion/react';
import { 
  ShieldCheck, 
  ArrowRight, 
  Lock, 
  Mail, 
  User, 
  Building2, 
  CheckCircle2, 
  Zap, 
  Check,
  Sparkles
} from 'lucide-react';
import { Button, Card, Input, Badge } from '@/components/ui';
import { Logo } from '@/components/Logo';
import { useTranslation } from '@/i18n/I18nContext';
import { LanguageSwitcher } from '@/components/LanguageSwitcher';

export function Register() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const [fullName, setFullName] = useState('Wei Huang');
  const [email, setEmail] = useState('wei@burncloud.io');
  const [companyName, setCompanyName] = useState('BurnCloud AI Labs');
  const [password, setPassword] = useState('••••••••••••');
  const [selectedTier, setSelectedTier] = useState<'developer' | 'growth' | 'enterprise'>('growth');
  const [termsAccepted, setTermsAccepted] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!termsAccepted) {
      alert("Please accept the Terms of Service to proceed.");
      return;
    }

    setIsLoading(true);
    setStatusMessage(t.publicPages.register.provisioning);

    setTimeout(() => {
      setIsLoading(false);
      navigate('/');
    }, 700);
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
          <span>{t.publicPages.register.alreadyHaveAccount}</span>
          <Link to="/login" className="font-semibold text-gray-900 hover:underline">
            {t.publicPages.register.signIn}
          </Link>
        </div>
      </header>

      {/* Main Registration Card */}
      <main className="flex-1 flex items-center justify-center p-6 my-auto">
        <div className="w-full max-w-xl space-y-6">
          <motion.div 
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            className="text-center space-y-2"
          >
            <div className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-green-50 border border-green-100 text-green-700 text-[11px] font-medium font-mono mb-1">
              <Sparkles className="w-3.5 h-3.5 text-green-600" />
              <span>{t.publicPages.register.includesCredits}</span>
            </div>
            <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-gray-950">
              {t.publicPages.register.title}
            </h1>
            <p className="text-xs sm:text-sm text-gray-500 max-w-md mx-auto">
              {t.publicPages.register.subtitle}
            </p>
          </motion.div>

          <Card className="p-8 space-y-6 shadow-xl border-gray-200/80 bg-white">
            <form onSubmit={handleSubmit} className="space-y-5">
              
              {/* Tier Selection Radio Pills */}
              <div className="space-y-2">
                <label className="text-xs font-semibold text-gray-700 block">
                  {t.publicPages.register.selectAccountType}
                </label>
                <div className="grid grid-cols-3 gap-3">
                  {[
                    { id: 'sandbox', name: t.publicPages.register.freeSandbox, price: '$0 Free', detail: '2M Test Tokens' },
                    { id: 'payg', name: t.publicPages.register.payAsYouGo, price: 'Pay Per Token', detail: '$5 Free Credits', popular: true },
                    { id: 'enterprise', name: t.publicPages.register.enterprise, price: 'Volume Rate', detail: 'Post-paid Invoice' }
                  ].map((tier) => (
                    <button
                      type="button"
                      key={tier.id}
                      onClick={() => setSelectedTier(tier.id as any)}
                      className={`p-3 rounded-xl border text-left transition-all cursor-pointer relative ${
                        selectedTier === (tier.id as any) || (selectedTier === 'growth' && tier.id === 'payg') || (selectedTier === 'developer' && tier.id === 'sandbox')
                          ? 'border-gray-900 bg-gray-50 shadow-sm ring-1 ring-gray-900' 
                          : 'border-gray-200 hover:border-gray-300 bg-white'
                      }`}
                    >
                      {tier.popular && (
                        <span className="absolute -top-2 right-2 bg-gray-900 text-white text-[9px] font-bold px-1.5 py-0.2 rounded uppercase font-mono">
                          POPULAR
                        </span>
                      )}
                      <div className="text-xs font-bold text-gray-950">{tier.name}</div>
                      <div className="text-[11px] font-mono text-gray-600 font-semibold mt-0.5">{tier.price}</div>
                      <div className="text-[10px] text-gray-400 mt-1">{tier.detail}</div>
                    </button>
                  ))}
                </div>
              </div>

              {/* Form Grid */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-gray-700 block">
                    {t.publicPages.register.fullName}
                  </label>
                  <div className="relative">
                    <User className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                    <Input
                      type="text"
                      required
                      value={fullName}
                      onChange={(e) => setFullName(e.target.value)}
                      placeholder="Jane Doe"
                      className="pl-9 text-xs"
                    />
                  </div>
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-gray-700 block">
                    {t.publicPages.register.companyTeam}
                  </label>
                  <div className="relative">
                    <Building2 className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                    <Input
                      type="text"
                      required
                      value={companyName}
                      onChange={(e) => setCompanyName(e.target.value)}
                      placeholder="Acme Corp"
                      className="pl-9 text-xs"
                    />
                  </div>
                </div>
              </div>

              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-gray-700 block">
                  {t.publicPages.register.workEmail}
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
                <label className="text-xs font-semibold text-gray-700 block">
                  {t.publicPages.register.password}
                </label>
                <div className="relative">
                  <Lock className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                  <Input
                    type="password"
                    required
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="At least 8 characters"
                    className="pl-9 text-xs"
                  />
                </div>
              </div>

              {/* Terms Checkbox */}
              <div className="pt-2 space-y-2">
                <label className="flex items-start gap-2.5 cursor-pointer text-xs text-gray-600 select-none">
                  <input
                    type="checkbox"
                    checked={termsAccepted}
                    onChange={(e) => setTermsAccepted(e.target.checked)}
                    className="mt-0.5 rounded border-gray-300 text-gray-900 focus:ring-gray-900"
                  />
                  <span className="leading-normal">
                    {t.publicPages.register.agreeTerms}
                  </span>
                </label>
              </div>

              {statusMessage && (
                <div className="p-3 bg-green-50 border border-green-100 rounded-xl text-xs text-green-800 flex items-center gap-2 font-mono">
                  <div className="w-3.5 h-3.5 border-2 border-green-600 border-t-transparent rounded-full animate-spin shrink-0" />
                  <span>{statusMessage}</span>
                </div>
              )}

              <Button
                type="submit"
                disabled={isLoading}
                className="w-full h-10 gap-2 text-xs font-semibold"
              >
                {isLoading ? t.common.loading : t.publicPages.register.createAccountBtn}
                {!isLoading && <ArrowRight className="w-4 h-4" />}
              </Button>
            </form>

            {/* Quick Demo Signup */}
            <div className="pt-4 border-t border-gray-100 text-center space-y-2">
              <Button
                variant="secondary"
                onClick={() => {
                  setIsLoading(true);
                  setTimeout(() => navigate('/'), 400);
                }}
                className="w-full h-9 text-xs gap-1.5 text-gray-700"
              >
                <Zap className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
                <span>{t.publicPages.register.instantDemoReg}</span>
              </Button>
            </div>
          </Card>

          <p className="text-center text-[11px] text-gray-400 font-mono">
            BurnCloud Gateway • Silicon Attested Multi-Cloud Infrastructure
          </p>
        </div>
      </main>

      {/* Footer */}
      <footer className="p-6 text-center text-xs text-gray-400">
        <div className="flex justify-center items-center gap-4">
          <Link to="/home" className="hover:text-gray-600">Home</Link>
          <span>•</span>
          <Link to="/login" className="hover:text-gray-600">Sign In</Link>
          <span>•</span>
          <a href="#privacy" className="hover:text-gray-600">Privacy Policy</a>
          <span>•</span>
          <a href="#terms" className="hover:text-gray-600">Terms of Service</a>
        </div>
      </footer>
    </div>
  );
}
