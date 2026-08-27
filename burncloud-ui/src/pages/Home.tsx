import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'motion/react';
import { 
  ShieldCheck, 
  ArrowRight, 
  Terminal, 
  Cpu, 
  Zap, 
  CheckCircle2, 
  Server, 
  Activity, 
  DollarSign, 
  Sparkles,
  ChevronRight,
  Check,
  Building2
} from 'lucide-react';
import { Button, Card, Badge } from '@/components/ui';
import { Logo } from '@/components/Logo';
import { useTranslation } from '@/i18n/I18nContext';
import { LanguageSwitcher } from '@/components/LanguageSwitcher';

export function Home() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const [activeModel, setActiveModel] = useState('claude-fable-5');
  const [simulatedLatency, setSimulatedLatency] = useState(142);
  const [, setIsSimulating] = useState(false);

  const simulateRoute = (modelName: string) => {
    setActiveModel(modelName);
    setIsSimulating(true);
    setSimulatedLatency(Math.floor(120 + Math.random() * 80));
    setTimeout(() => {
      setIsSimulating(false);
    }, 400);
  };

  return (
    <div className="min-h-screen bg-[#F9FAFB] text-gray-900 font-sans selection:bg-gray-900 selection:text-white flex flex-col">
      {/* Top Header Navbar */}
      <header className="sticky top-0 z-50 bg-white/80 backdrop-blur-md border-b border-gray-200/60 transition-all">
        <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-8">
            <Link to="/home" className="flex items-center gap-2.5 group">
              <Logo className="w-7 h-7 group-hover:scale-105 transition-transform" />
              <span className="font-display font-bold text-lg tracking-tight text-gray-900">BurnCloud</span>
              <Badge variant="brand" className="hidden sm:inline-flex text-[10px] px-2 py-0">GATEWAY v2.4</Badge>
            </Link>

            <nav className="hidden md:flex items-center gap-6 text-[13px] font-medium text-gray-600">
              <a href="#features" className="hover:text-gray-900 transition-colors">{t.publicPages.home.featuresTitle}</a>
              <a href="#architecture" className="hover:text-gray-900 transition-colors">{t.publicPages.home.architecture}</a>
              <a href="#pricing" className="hover:text-gray-900 transition-colors">{t.publicPages.home.pricing}</a>
              <Link to="/buyer/playground" className="hover:text-gray-900 transition-colors flex items-center gap-1">
                {t.nav.playground} <span className="text-[10px] bg-amber-100 text-amber-800 px-1.5 py-0.2 rounded font-mono font-bold">{t.common.live}</span>
              </Link>
            </nav>
          </div>

          <div className="flex items-center gap-3">
            <LanguageSwitcher variant="navbar" />
            <Link to="/login">
              <Button variant="ghost" className="text-[13px]">
                {t.publicPages.home.signIn}
              </Button>
            </Link>
            <Link to="/register">
              <Button variant="primary" className="text-[13px] gap-1.5">
                <span>{t.publicPages.home.startTrial}</span>
                <ArrowRight className="w-3.5 h-3.5" />
              </Button>
            </Link>
            <Link to="/" className="hidden lg:inline-flex">
              <Button variant="secondary" className="text-[13px] gap-1.5">
                <span>{t.publicPages.home.goToConsole}</span>
                <ChevronRight className="w-3.5 h-3.5" />
              </Button>
            </Link>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="relative pt-16 pb-20 md:pt-24 md:pb-28 overflow-hidden bg-gradient-to-b from-white via-[#F9FAFB] to-[#F9FAFB]">
        <div className="absolute inset-0 bg-[linear-gradient(to_right,#8080800a_1px,transparent_1px),linear-gradient(to_bottom,#8080800a_1px,transparent_1px)] bg-[size:24px_24px] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,#000_70%,transparent_100%)] pointer-events-none" />

        <div className="max-w-7xl mx-auto px-6 relative z-10 text-center">
          {/* Trust Pill */}
          <motion.div 
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-gray-100 border border-gray-200/80 text-[12px] font-medium text-gray-700 mb-6 shadow-sm"
          >
            <span className="flex h-2 w-2 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
            </span>
            <span className="font-mono font-semibold">{t.publicPages.home.trustPill}</span>
          </motion.div>

          {/* Hero Headline */}
          <motion.h1 
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="text-4xl sm:text-6xl lg:text-7xl font-bold tracking-tight text-gray-950 max-w-4xl mx-auto leading-[1.1]"
          >
            {t.publicPages.home.heroTitle}
          </motion.h1>

          <motion.p 
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="mt-6 text-lg sm:text-xl text-gray-600 max-w-2xl mx-auto leading-relaxed font-normal"
          >
            {t.publicPages.home.heroSubtitle}
          </motion.p>

          {/* Hero CTAs */}
          <motion.div 
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
            className="mt-8 flex flex-wrap items-center justify-center gap-4"
          >
            <Button 
              size="lg" 
              onClick={() => navigate('/register')}
              className="gap-2 text-[15px] px-7 py-3 shadow-lg shadow-gray-900/10"
            >
              <Zap className="w-4 h-4 fill-amber-400 text-amber-400" />
              <span>{t.publicPages.home.deployFree}</span>
            </Button>
            
            <Button 
              size="lg" 
              variant="secondary"
              onClick={() => navigate('/')}
              className="gap-2 text-[15px] px-6"
            >
              <Terminal className="w-4 h-4 text-gray-600" />
              <span>{t.publicPages.home.openConsole}</span>
            </Button>
          </motion.div>

          {/* Stat Pill Strip */}
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.4 }}
            className="mt-14 pt-8 border-t border-gray-200/60 max-w-4xl mx-auto grid grid-cols-2 md:grid-cols-4 gap-6 text-left"
          >
            <div>
              <div className="text-2xl font-bold text-gray-950 font-sans tracking-tight">12.8M+</div>
              <div className="text-xs text-gray-500 font-medium mt-0.5">{t.publicPages.home.statsRequests}</div>
            </div>
            <div>
              <div className="text-2xl font-bold text-gray-950 font-sans tracking-tight">142ms</div>
              <div className="text-xs text-gray-500 font-medium mt-0.5">{t.publicPages.home.statsLatency}</div>
            </div>
            <div>
              <div className="text-2xl font-bold text-gray-950 font-sans tracking-tight">99.999%</div>
              <div className="text-xs text-gray-500 font-medium mt-0.5">{t.publicPages.home.statsUptime}</div>
            </div>
            <div>
              <div className="text-2xl font-bold text-gray-950 font-sans tracking-tight">$4,766</div>
              <div className="text-xs text-gray-500 font-medium mt-0.5">{t.publicPages.home.statsSavings}</div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Interactive Gateway Demo Card */}
      <section className="py-12 bg-white border-y border-gray-200/60">
        <div className="max-w-6xl mx-auto px-6">
          <div className="text-center mb-8">
            <Badge variant="brand" className="mb-2 uppercase tracking-wider">{t.publicPages.home.liveDemoBadge}</Badge>
            <h2 className="text-2xl sm:text-3xl font-bold text-gray-950">
              {t.publicPages.home.liveDemoTitle}
            </h2>
            <p className="text-sm text-gray-500 mt-1 max-w-xl mx-auto">
              {t.publicPages.home.liveDemoSubtitle}
            </p>
          </div>

          <Card className="p-6 md:p-8 bg-gray-950 text-white shadow-2xl rounded-2xl overflow-hidden border-gray-800 relative">
            {/* Model Selector Pills */}
            <div className="flex flex-wrap items-center justify-between gap-4 pb-6 border-b border-gray-800">
              <div className="flex items-center gap-2">
                <span className="text-xs text-gray-400 font-mono uppercase tracking-wider">{t.publicPages.home.selectModel}</span>
                {['claude-fable-5', 'gpt-4o-nitro', 'deepseek-r1-enclave', 'llama-3.3-70b'].map((model) => (
                  <button
                    key={model}
                    onClick={() => simulateRoute(model)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-mono transition-all cursor-pointer ${
                      activeModel === model 
                        ? 'bg-blue-600 text-white font-bold shadow-md shadow-blue-500/20' 
                        : 'bg-gray-900 text-gray-400 hover:text-white hover:bg-gray-800'
                    }`}
                  >
                    {model}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-3">
                <div className="flex items-center gap-1.5 text-xs font-mono text-green-400 bg-green-950/60 border border-green-800/60 px-3 py-1 rounded-full">
                  <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
                  <span>{t.publicPages.home.latencyLabel}: {simulatedLatency}ms</span>
                </div>
                <div className="flex items-center gap-1.5 text-xs font-mono text-blue-400 bg-blue-950/60 border border-blue-800/60 px-3 py-1 rounded-full">
                  <ShieldCheck className="w-3.5 h-3.5" />
                  <span>{t.publicPages.home.tpmSignedLabel}</span>
                </div>
              </div>
            </div>

            {/* Code / JSON Display */}
            <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* Left Column: Request */}
              <div className="space-y-3">
                <div className="flex items-center justify-between text-xs font-mono text-gray-400">
                  <span className="flex items-center gap-1.5">
                    <Terminal className="w-3.5 h-3.5 text-amber-400" />
                    {t.publicPages.home.curlSnippetTitle}
                  </span>
                  <span className="text-green-400">HTTP 200 OK</span>
                </div>
                <pre className="bg-gray-900 p-4 rounded-xl text-xs font-mono text-gray-300 overflow-x-auto border border-gray-800 leading-relaxed">
{`curl -X POST https://gateway.burncloud.io/v1/chat \\
  -H "Authorization: Bearer demo-api-key" \\
  -H "X-BurnCloud-Attest: TPM_ENCLAVE_STRICT" \\
  -d '{
    "model": "${activeModel}",
    "messages": [{"role": "user", "content": "Ping"}]
  }'`}
                </pre>
              </div>

              {/* Right Column: Hardware Receipt */}
              <div className="space-y-3">
                <div className="flex items-center justify-between text-xs font-mono text-gray-400">
                  <span className="flex items-center gap-1.5">
                    <ShieldCheck className="w-3.5 h-3.5 text-green-400" />
                    {t.publicPages.home.returnedReceiptTitle}
                  </span>
                  <span className="text-blue-400">{t.publicPages.home.verifiedSiliconBadge}</span>
                </div>
                <pre className="bg-gray-900 p-4 rounded-xl text-xs font-mono text-gray-300 overflow-x-auto border border-gray-800 leading-relaxed">
{`{
  "burncloud_trace_id": "tr_8f3c11_${activeModel.replace(/-/g, '_')}",
  "hardware_signature": "0x98f42ba7...enclave",
  "routed_provider": "AWS Bedrock (us-east-1)",
  "latency_ms": ${simulatedLatency},
  "dilution_check": "PASSED (0% middleman variance)"
}`}
                </pre>
              </div>
            </div>

            {/* Bottom Bar */}
            <div className="mt-6 pt-4 border-t border-gray-800 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-gray-400">
              <div className="flex items-center gap-2">
                <Sparkles className="w-4 h-4 text-amber-400 animate-pulse" />
                <span>{t.publicPages.home.nitroGuaranteedText}</span>
              </div>
              <Link to="/buyer/playground" className="text-blue-400 hover:text-blue-300 font-medium flex items-center gap-1">
                {t.publicPages.home.openPlaygroundLink}
              </Link>
            </div>
          </Card>
        </div>
      </section>

      {/* Value Pillars Section */}
      <section id="features" className="py-20 bg-[#F9FAFB]">
        <div className="max-w-7xl mx-auto px-6">
          <div className="text-center max-w-3xl mx-auto mb-16">
            <h2 className="text-xs font-semibold text-gray-400 uppercase tracking-widest font-mono mb-2">
              {t.publicPages.home.whyBurncloudEyebrow}
            </h2>
            <p className="text-3xl sm:text-4xl font-bold text-gray-950 tracking-tight">
              {t.publicPages.home.whyBurncloudTitle}
            </p>
            <p className="text-gray-600 mt-3 text-base">
              {t.publicPages.home.whyBurncloudSubtitle}
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            <Card className="p-7 space-y-4 hover:border-gray-300 transition-all duration-200 hover:-translate-y-1">
              <div className="w-12 h-12 bg-blue-50 border border-blue-100 rounded-2xl flex items-center justify-center text-blue-600">
                <ShieldCheck className="w-6 h-6" />
              </div>
              <h3 className="text-lg font-bold text-gray-950">{t.publicPages.home.pillar1Title}</h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                {t.publicPages.home.pillar1Desc}
              </p>
              <ul className="space-y-2 pt-2 text-xs text-gray-500 font-mono">
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar1Item1}</li>
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar1Item2}</li>
              </ul>
            </Card>

            <Card className="p-7 space-y-4 hover:border-gray-300 transition-all duration-200 hover:-translate-y-1">
              <div className="w-12 h-12 bg-purple-50 border border-purple-100 rounded-2xl flex items-center justify-center text-purple-600">
                <Zap className="w-6 h-6" />
              </div>
              <h3 className="text-lg font-bold text-gray-950">{t.publicPages.home.pillar2Title}</h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                {t.publicPages.home.pillar2Desc}
              </p>
              <ul className="space-y-2 pt-2 text-xs text-gray-500 font-mono">
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar2Item1}</li>
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar2Item2}</li>
              </ul>
            </Card>

            <Card className="p-7 space-y-4 hover:border-gray-300 transition-all duration-200 hover:-translate-y-1">
              <div className="w-12 h-12 bg-amber-50 border border-amber-100 rounded-2xl flex items-center justify-center text-amber-600">
                <DollarSign className="w-6 h-6" />
              </div>
              <h3 className="text-lg font-bold text-gray-950">{t.publicPages.home.pillar3Title}</h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                {t.publicPages.home.pillar3Desc}
              </p>
              <ul className="space-y-2 pt-2 text-xs text-gray-500 font-mono">
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar3Item1}</li>
                <li className="flex items-center gap-2"><CheckCircle2 className="w-3.5 h-3.5 text-green-600" /> {t.publicPages.home.pillar3Item2}</li>
              </ul>
            </Card>
          </div>
        </div>
      </section>

      {/* Architecture & Provider Matrix */}
      <section id="architecture" className="py-20 bg-white border-t border-gray-200/60">
        <div className="max-w-7xl mx-auto px-6">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
            <div className="space-y-6">
              <Badge variant="brand">{t.publicPages.home.matrixBadge}</Badge>
              <h2 className="text-3xl sm:text-4xl font-bold text-gray-950 tracking-tight leading-tight">
                {t.publicPages.home.matrixTitle}
              </h2>
              <p className="text-gray-600 text-base leading-relaxed">
                {t.publicPages.home.matrixDesc}
              </p>

              <div className="space-y-4">
                {[
                  { title: t.publicPages.home.matrixProvider1Title, desc: t.publicPages.home.matrixProvider1Desc, icon: Server },
                  { title: t.publicPages.home.matrixProvider2Title, desc: t.publicPages.home.matrixProvider2Desc, icon: Cpu },
                  { title: t.publicPages.home.matrixProvider3Title, desc: t.publicPages.home.matrixProvider3Desc, icon: Building2 }
                ].map((item, idx) => (
                  <div key={idx} className="flex items-start gap-3.5 p-4 rounded-xl bg-gray-50 border border-gray-200/60">
                    <div className="p-2 bg-white rounded-lg border border-gray-200/80 text-gray-900 shrink-0 mt-0.5">
                      <item.icon className="w-5 h-5" />
                    </div>
                    <div>
                      <h4 className="text-sm font-bold text-gray-950">{item.title}</h4>
                      <p className="text-xs text-gray-500 mt-0.5">{item.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <Card className="p-6 bg-[#F9FAFB] border-gray-200/80 space-y-6">
              <div className="flex items-center justify-between border-b border-gray-200 pb-4">
                <div className="flex items-center gap-2">
                  <Activity className="w-5 h-5 text-blue-600" />
                  <span className="font-bold text-sm text-gray-900">{t.publicPages.home.activeRoutingNodes}</span>
                </div>
                <Badge variant="success">{t.publicPages.home.allSystemsNominal}</Badge>
              </div>

              <div className="space-y-4 font-mono text-xs">
                {[
                  { name: "aws-bedrock-us-east-1", status: "ONLINE", ms: "124ms", load: "48%" },
                  { name: "anthropic-direct-cluster", status: "ONLINE", ms: "148ms", load: "32%" },
                  { name: "gcp-vertex-us-central1", status: "ONLINE", ms: "135ms", load: "16%" },
                  { name: "groq-accelerator-node", status: "ONLINE", ms: "89ms", load: "4%" }
                ].map((node, i) => (
                  <div key={i} className="p-3 bg-white rounded-xl border border-gray-200/80 flex items-center justify-between">
                    <div className="flex items-center gap-2.5">
                      <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
                      <span className="font-semibold text-gray-900">{node.name}</span>
                    </div>
                    <div className="flex items-center gap-3 text-gray-500">
                      <span>{node.ms}</span>
                      <span className="bg-gray-100 text-gray-700 px-2 py-0.5 rounded text-[10px] font-bold">{node.load}</span>
                    </div>
                  </div>
                ))}
              </div>

              <div className="pt-4 border-t border-gray-200 text-center">
                <Link to="/buyer/marketplace" className="text-xs font-semibold text-blue-600 hover:text-blue-700 inline-flex items-center gap-1">
                  {t.publicPages.home.manageRoutesLink}
                </Link>
              </div>
            </Card>
          </div>
        </div>
      </section>

      {/* Pricing Section - Pure Token Pay-As-You-Go */}
      <section id="pricing" className="py-20 bg-[#F9FAFB]">
        <div className="max-w-7xl mx-auto px-6">
          <div className="text-center max-w-2xl mx-auto mb-12">
            <Badge variant="brand" className="mb-2 uppercase tracking-widest font-mono">{t.publicPages.home.pricingBadge}</Badge>
            <p className="text-3xl sm:text-4xl font-bold text-gray-950 tracking-tight">{t.publicPages.home.pricingTitle}</p>
            <p className="text-gray-600 mt-2 text-sm leading-relaxed">
              {t.publicPages.home.pricingSubtitle}
            </p>

            {/* Token Transparency Banner */}
            <div className="mt-6 inline-flex items-center gap-2 px-4 py-2 bg-white border border-gray-200/80 rounded-full shadow-sm text-xs font-mono text-gray-700">
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
              <span className="font-semibold text-gray-900">{t.publicPages.home.directPassThrough}</span>
              <span className="text-gray-300">•</span>
              <span className="text-green-700 font-bold">{t.publicPages.home.zeroTokenMarkup}</span>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-8 max-w-6xl mx-auto">
            {/* Developer Sandbox */}
            <Card className="p-8 space-y-6 flex flex-col justify-between hover:border-gray-300 transition-all">
              <div className="space-y-4">
                <Badge variant="neutral">{t.publicPages.home.sandboxTitle}</Badge>
                <div>
                  <span className="text-4xl font-bold text-gray-950">{t.publicPages.home.sandboxPrice}</span>
                  <span className="text-gray-500 text-xs font-medium block mt-1">{t.publicPages.home.sandboxSubtitle}</span>
                </div>
                <p className="text-xs text-gray-600">{t.publicPages.home.sandboxDesc}</p>
                <div className="pt-4 border-t border-gray-100 space-y-3 text-xs text-gray-700">
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.sandboxItem1}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.sandboxItem2}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.sandboxItem3}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.sandboxItem4}</div>
                </div>
              </div>
              <Button onClick={() => navigate('/register')} variant="secondary" className="w-full">
                {t.publicPages.home.sandboxBtn}
              </Button>
            </Card>

            {/* Pay-As-You-Go Standard (Featured) */}
            <Card className="p-8 space-y-6 flex flex-col justify-between border-2 border-gray-900 shadow-xl relative bg-white">
              <div className="absolute -top-3 left-1/2 -translate-x-1/2 bg-gray-900 text-white text-[10px] font-bold px-3 py-0.5 rounded-full uppercase tracking-wider font-mono">
                {t.publicPages.home.popularBadge}
              </div>
              <div className="space-y-4">
                <Badge variant="brand">{t.publicPages.home.paygTitle}</Badge>
                <div>
                  <span className="text-3xl font-bold text-gray-950">{t.publicPages.home.paygPrice}</span>
                  <span className="text-gray-500 text-xs font-medium block mt-1">{t.publicPages.home.paygSubtitle}</span>
                </div>
                <p className="text-xs text-gray-600">{t.publicPages.home.paygDesc}</p>
                <div className="pt-4 border-t border-gray-100 space-y-3 text-xs text-gray-700">
                  <div className="flex items-center gap-2 font-semibold"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.paygItem1}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.paygItem2}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.paygItem3}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.paygItem4}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.paygItem5}</div>
                </div>
              </div>
              <Button onClick={() => navigate('/register')} variant="primary" className="w-full gap-2">
                <span>{t.publicPages.home.paygBtn}</span>
                <ArrowRight className="w-4 h-4" />
              </Button>
            </Card>

            {/* Enterprise Volume */}
            <Card className="p-8 space-y-6 flex flex-col justify-between hover:border-gray-300 transition-all">
              <div className="space-y-4">
                <Badge variant="neutral">{t.publicPages.home.enterpriseTitle}</Badge>
                <div>
                  <span className="text-3xl font-bold text-gray-950">{t.publicPages.home.enterprisePrice}</span>
                  <span className="text-gray-500 text-xs font-medium block mt-1">{t.publicPages.home.enterpriseSubtitle}</span>
                </div>
                <p className="text-xs text-gray-600">{t.publicPages.home.enterpriseDesc}</p>
                <div className="pt-4 border-t border-gray-100 space-y-3 text-xs text-gray-700">
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.enterpriseItem1}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.enterpriseItem2}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.enterpriseItem3}</div>
                  <div className="flex items-center gap-2"><Check className="w-4 h-4 text-green-600" /> {t.publicPages.home.enterpriseItem4}</div>
                </div>
              </div>
              <Button onClick={() => navigate('/register')} variant="secondary" className="w-full">
                {t.publicPages.home.enterpriseBtn}
              </Button>
            </Card>
          </div>

          {/* Model Token Pass-Through Price Sample */}
          <div className="mt-12 max-w-4xl mx-auto p-6 bg-white border border-gray-200/80 rounded-2xl shadow-sm">
            <div className="flex flex-col sm:flex-row items-center justify-between gap-4 mb-4 pb-4 border-b border-gray-100">
              <div>
                <h4 className="text-sm font-bold text-gray-950">{t.publicPages.home.sampleRatesTitle}</h4>
                <p className="text-xs text-gray-500">{t.publicPages.home.sampleRatesSubtitle}</p>
              </div>
              <Badge variant="success" className="font-mono text-[10px]">{t.publicPages.home.verifiedZeroMarkup}</Badge>
            </div>
            
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 text-xs font-mono">
              <div className="p-3 bg-gray-50 rounded-xl">
                <div className="text-gray-500 text-[10px]">Claude Fable 5</div>
                <div className="font-bold text-gray-900 mt-1">$3.00 / 1M in</div>
                <div className="text-gray-600">$15.00 / 1M out</div>
              </div>
              <div className="p-3 bg-gray-50 rounded-xl">
                <div className="text-gray-500 text-[10px]">GPT-4o Nitro</div>
                <div className="font-bold text-gray-900 mt-1">$2.50 / 1M in</div>
                <div className="text-gray-600">$10.00 / 1M out</div>
              </div>
              <div className="p-3 bg-gray-50 rounded-xl">
                <div className="text-gray-500 text-[10px]">DeepSeek R1</div>
                <div className="font-bold text-gray-900 mt-1">$0.55 / 1M in</div>
                <div className="text-gray-600">$2.19 / 1M out</div>
              </div>
              <div className="p-3 bg-gray-50 rounded-xl">
                <div className="text-gray-500 text-[10px]">Llama 3.3 70B</div>
                <div className="font-bold text-gray-900 mt-1">$0.60 / 1M in</div>
                <div className="text-gray-600">$0.60 / 1M out</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="mt-auto bg-white border-t border-gray-200/80 py-12 text-xs text-gray-500">
        <div className="max-w-7xl mx-auto px-6 flex flex-col md:flex-row justify-between items-center gap-6">
          <div className="flex items-center gap-2.5">
            <Logo className="w-6 h-6" />
            <span className="font-bold text-gray-900 text-sm">BurnCloud Gateway</span>
            <span className="text-gray-300">•</span>
            <span>{t.publicPages.home.footerCopyright}</span>
          </div>

          <div className="flex items-center gap-6 font-medium">
            <Link to="/home" className="hover:text-gray-900 transition-colors">{t.common.publicPortal}</Link>
            <Link to="/login" className="hover:text-gray-900 transition-colors">{t.common.signIn}</Link>
            <Link to="/register" className="hover:text-gray-900 transition-colors">{t.common.register}</Link>
            <Link to="/" className="hover:text-gray-900 transition-colors">{t.common.console}</Link>
            <Link to="/buyer/playground" className="hover:text-gray-900 transition-colors">{t.nav.playground}</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
