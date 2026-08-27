import React, { useState } from 'react';
import { NavLink, useLocation, Link, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard,
  Terminal,
  Store,
  Key,
  LineChart,
  CreditCard,
  ScrollText,
  Server,
  Layers,
  Coins,
  Receipt,
  ShieldCheck,
  Settings,
  Cpu,
  Gauge,
  TrendingUp,
  Boxes,
  Users,
  Building2,
  Workflow,
  Search,
  Bell,
  HelpCircle,
  Globe,
  ChevronDown,
  Sparkles,
  Zap,
  Shield,
  Sliders,
  DollarSign
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Logo } from './Logo';
import { useRole, UserRole } from '@/context/RoleContext';
import { useTranslation } from '@/i18n/I18nContext';
import { LanguageSwitcher } from './LanguageSwitcher';
import { BCBadge, BCButton } from './ui';

interface NavItem {
  name: string;
  path: string;
  icon: React.ElementType;
  badge?: string;
}

interface NavSection {
  title?: string;
  items: NavItem[];
}

export function Layout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const {
    role,
    setRole,
    balance,
    todaySpend,
    supplierEarningsToday,
    adminRevenueToday,
    notificationsCount,
    searchQuery,
    setSearchQuery
  } = useRole();

  const [roleDropdownOpen, setRoleDropdownOpen] = useState(false);
  const [quickTopUpOpen, setQuickTopUpOpen] = useState(false);

  const isPublicPage = ['/home', '/landing', '/login', '/register'].includes(location.pathname);

  if (isPublicPage) {
    return <>{children}</>;
  }

  // 1. Role-specific Navigation Groups
  const buyerNavSections: NavSection[] = [
    {
      items: [
        { name: t.nav.overview, path: '/buyer/overview', icon: LayoutDashboard },
        { name: t.nav.playground, path: '/buyer/playground', icon: Terminal, badge: t.common.live },
        { name: t.nav.marketplace, path: '/buyer/marketplace', icon: Store },
        { name: t.nav.apiKeys, path: '/buyer/api-keys', icon: Key },
        { name: t.nav.usage, path: '/buyer/usage', icon: LineChart },
        { name: t.nav.billing, path: '/buyer/billing', icon: CreditCard },
        { name: t.nav.logs, path: '/buyer/logs', icon: ScrollText },
      ]
    }
  ];

  const supplierNavSections: NavSection[] = [
    {
      items: [
        { name: t.nav.overview, path: '/supplier/overview', icon: LayoutDashboard },
        { name: t.nav.resources, path: '/supplier/resources', icon: Server, badge: '4 Nodes' },
        { name: t.nav.deployments, path: '/supplier/deployments', icon: Layers },
        { name: t.nav.earnings, path: '/supplier/earnings', icon: Coins },
        { name: t.nav.settlements, path: '/supplier/settlements', icon: Receipt },
        { name: t.nav.reliability, path: '/supplier/reliability', icon: ShieldCheck },
        { name: t.nav.settings, path: '/supplier/settings', icon: Settings },
      ]
    }
  ];

  const adminNavSections: NavSection[] = [
    {
      title: t.nav.platformCommand,
      items: [
        { name: t.nav.overview, path: '/admin/overview', icon: LayoutDashboard },
        { name: t.nav.supply, path: '/admin/supply', icon: Server },
        { name: t.nav.capacity, path: '/admin/capacity', icon: Gauge, badge: t.common.auto },
        { name: t.nav.demand, path: '/admin/demand', icon: TrendingUp },
        { name: t.nav.models, path: '/admin/models', icon: Cpu },
      ]
    },
    {
      title: t.nav.economicsAndOps,
      items: [
        { name: t.nav.revenue, path: '/admin/revenue', icon: DollarSign },
        { name: t.nav.settlements, path: '/admin/settlements', icon: Receipt },
        { name: t.nav.suppliers, path: '/admin/suppliers', icon: Building2 },
        { name: t.nav.customers, path: '/admin/customers', icon: Users },
        { name: t.nav.operations, path: '/admin/operations', icon: Workflow, badge: 'Autopilot' },
        { name: t.nav.settings, path: '/admin/settings', icon: Settings },
      ]
    }
  ];

  const currentNavSections =
    role === 'buyer'
      ? buyerNavSections
      : role === 'supplier'
      ? supplierNavSections
      : adminNavSections;

  const roleMeta = {
    buyer: {
      label: t.roles.buyer.title,
      subtext: t.roles.buyer.subtext,
      flowText: t.roles.buyer.flowText,
      color: 'bg-emerald-500',
      badgeColor: 'success' as const,
      metricLabel: t.common.balance,
      metricValue: `$${balance.toFixed(2)}`,
    },
    supplier: {
      label: t.roles.supplier.title,
      subtext: t.roles.supplier.subtext,
      flowText: t.roles.supplier.flowText,
      color: 'bg-indigo-500',
      badgeColor: 'accent' as const,
      metricLabel: t.supplier.overview.todayEarnings,
      metricValue: `$${supplierEarningsToday.toFixed(2)}`,
    },
    admin: {
      label: t.roles.admin.title,
      subtext: t.roles.admin.subtext,
      flowText: t.roles.admin.flowText,
      color: 'bg-amber-500',
      badgeColor: 'warning' as const,
      metricLabel: t.admin.overview.platformGmv,
      metricValue: `$${adminRevenueToday.toLocaleString()}`,
    }
  };

  return (
    <div className="flex h-screen bg-[#F9FAFB] text-gray-900 overflow-hidden font-sans select-none">
      {/* Sidebar */}
      <aside className="w-[230px] flex flex-col border-r border-gray-200/90 bg-[#F9FAFB] flex-shrink-0">
        {/* Brand & Role Switcher */}
        <div className="p-3 border-b border-gray-200/70">
          <div className="relative">
            <button
              onClick={() => setRoleDropdownOpen(!roleDropdownOpen)}
              className="w-full flex items-center justify-between p-2 rounded-xl bg-white border border-gray-200/90 hover:border-gray-300 shadow-xs transition-all text-left group"
            >
              <div className="flex items-center gap-2.5 min-w-0">
                <Logo className="w-6 h-6 flex-shrink-0" />
                <div className="min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="font-bold text-[13px] tracking-tight text-gray-950">BurnCloud</span>
                    <span className={cn("w-1.5 h-1.5 rounded-full flex-shrink-0", roleMeta[role].color)} />
                  </div>
                  <div className="text-[11px] font-mono font-medium text-gray-500 truncate flex items-center gap-1">
                    <span>{roleMeta[role].label}</span>
                    <span className="text-[9px] text-gray-400">• Workspace</span>
                  </div>
                </div>
              </div>
              <ChevronDown className="w-4 h-4 text-gray-400 group-hover:text-gray-700 transition-colors flex-shrink-0" />
            </button>

            {/* Role Dropdown */}
            {roleDropdownOpen && (
              <>
                <div
                  className="fixed inset-0 z-30"
                  onClick={() => setRoleDropdownOpen(false)}
                />
                <div className="absolute top-full left-0 right-0 mt-1.5 bg-white border border-gray-200 rounded-xl shadow-xl z-40 p-1.5 space-y-1">
                  <div className="px-2.5 py-1 text-[10px] font-mono font-bold uppercase tracking-wider text-gray-400">
                    {t.common.switchWorkspaceRole}
                  </div>
                  {(['buyer', 'supplier', 'admin'] as UserRole[]).map((r) => (
                    <button
                      key={r}
                      onClick={() => {
                        setRole(r);
                        setRoleDropdownOpen(false);
                      }}
                      className={cn(
                        "w-full flex items-center justify-between px-2.5 py-2 rounded-lg text-xs transition-colors text-left",
                        role === r
                          ? "bg-gray-900 text-white font-medium shadow-xs"
                          : "text-gray-700 hover:bg-gray-100"
                      )}
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className={cn(
                            "w-2 h-2 rounded-full",
                            roleMeta[r].color
                          )}
                        />
                        <div>
                          <div className="font-semibold">{roleMeta[r].label}</div>
                          <div className={cn("text-[10px]", role === r ? "text-gray-300" : "text-gray-400")}>
                            {roleMeta[r].subtext}
                          </div>
                        </div>
                      </div>
                      {role === r && (
                        <span className="text-[10px] font-mono uppercase bg-white/20 px-1.5 py-0.5 rounded">
                          {t.common.active}
                        </span>
                      )}
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
        </div>

        {/* Mental Model Banner Tag */}
        <div className="px-4 py-2 border-b border-gray-200/50 bg-gray-100/40">
          <div className="text-[10px] font-mono text-gray-500 uppercase tracking-wider">
            {roleMeta[role].flowText}
          </div>
        </div>

        {/* Nav Links */}
        <nav className="flex-1 overflow-y-auto p-2.5 space-y-4">
          {currentNavSections.map((section, idx) => (
            <div key={idx} className="space-y-0.5">
              {section.title && (
                <div className="px-3 pt-2 pb-1 text-[10px] font-mono font-bold uppercase tracking-wider text-gray-400">
                  {section.title}
                </div>
              )}
              {section.items.map((item) => {
                const Icon = item.icon;
                const isActive = location.pathname.startsWith(item.path);
                return (
                  <NavLink
                    key={item.path}
                    to={item.path}
                    className={cn(
                      "flex items-center justify-between px-3 py-1.5 rounded-xl text-xs font-medium transition-all group",
                      isActive
                        ? "bg-gray-900 text-white shadow-xs"
                        : "text-gray-600 hover:text-gray-950 hover:bg-gray-200/60"
                    )}
                  >
                    <div className="flex items-center gap-2.5">
                      <Icon
                        className={cn(
                          "w-4 h-4 transition-colors",
                          isActive ? "text-white" : "text-gray-400 group-hover:text-gray-700"
                        )}
                      />
                      <span>{item.name}</span>
                    </div>
                    {item.badge && (
                      <span
                        className={cn(
                          "text-[9px] font-mono font-bold px-1.5 py-0.5 rounded uppercase tracking-wider",
                          isActive
                            ? "bg-white/20 text-white"
                            : "bg-gray-200 text-gray-700 group-hover:bg-gray-300"
                        )}
                      >
                        {item.badge}
                      </span>
                    )}
                  </NavLink>
                );
              })}
            </div>
          ))}
        </nav>

        {/* Sidebar Footer */}
        <div className="p-3 border-t border-gray-200/80 bg-white/60 space-y-2">
          {/* Role Status Summary */}
          <div className="p-2.5 rounded-xl bg-gray-50 border border-gray-200/70 space-y-1">
            <div className="text-[10px] font-mono font-semibold uppercase text-gray-400">
              {roleMeta[role].metricLabel}
            </div>
            <div className="flex items-baseline justify-between">
              <span className="font-mono font-bold text-sm text-gray-950">
                {roleMeta[role].metricValue}
              </span>
              {role === 'buyer' && (
                <button
                  onClick={() => navigate('/buyer/billing')}
                  className="text-[10px] font-bold text-blue-600 hover:text-blue-700 font-mono hover:underline"
                >
                  + Top Up
                </button>
              )}
            </div>
          </div>

          <div className="flex items-center justify-between text-xs px-1 text-gray-500">
            <Link
              to="/home"
              className="flex items-center gap-1 hover:text-gray-900 transition-colors text-[11px]"
            >
              <Globe className="w-3.5 h-3.5" />
              <span>{t.common.publicPortal}</span>
            </Link>
            <span className="text-[10px] font-mono text-emerald-600 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
              {t.common.slaScore}
            </span>
          </div>
        </div>
      </aside>

      {/* Main App Container */}
      <div className="flex-1 flex flex-col min-w-0 bg-[#F9FAFB]">
        {/* Top Header */}
        <header className="h-14 bg-white border-b border-gray-200/90 px-6 flex items-center justify-between z-10 flex-shrink-0">
          {/* Left: Global Search & Breadcrumb */}
          <div className="flex items-center gap-4 flex-1 max-w-xl">
            <div className="relative w-full max-w-md">
              <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={
                  role === 'buyer'
                    ? t.common.searchPlaceholderBuyer
                    : role === 'supplier'
                    ? t.common.searchPlaceholderSupplier
                    : t.common.searchPlaceholderAdmin
                }
                className="w-full h-8 pl-8 pr-3 bg-gray-50 hover:bg-gray-100/80 focus:bg-white border border-gray-200/80 rounded-xl text-xs text-gray-900 placeholder:text-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-900/10 focus:border-gray-900 transition-all font-sans"
              />
            </div>
          </div>

          {/* Right Header Actions */}
          <div className="flex items-center gap-3">
            {/* Live Autopilot Status */}
            <div className="hidden sm:flex items-center gap-2 px-2.5 py-1 bg-gray-50 border border-gray-200/80 rounded-lg text-xs font-mono text-gray-700">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              <span className="text-[11px] font-semibold text-gray-800">{t.common.autopilotActive}</span>
            </div>

            {/* Language Switcher Dropdown */}
            <LanguageSwitcher variant="navbar" />

            {/* Needs Attention / Notification */}
            <button
              onClick={() => {
                if (role === 'buyer') navigate('/buyer/overview#attention');
                else if (role === 'supplier') navigate('/supplier/overview#attention');
                else navigate('/admin/overview#attention');
              }}
              className="relative p-2 text-gray-500 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
              title={t.common.attentionNeeded}
            >
              <Bell className="w-4 h-4" />
              {notificationsCount > 0 && (
                <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-amber-500 rounded-full ring-2 ring-white" />
              )}
            </button>

            <div className="h-4 w-px bg-gray-200 mx-1" />

            {/* Profile Avatar */}
            <div className="flex items-center gap-2 pl-1">
              <div className="w-7 h-7 rounded-full bg-gray-900 text-white font-mono font-bold text-xs flex items-center justify-center shadow-xs">
                {role === 'buyer' ? 'BY' : role === 'supplier' ? 'SP' : 'AD'}
              </div>
              <div className="hidden md:block text-left">
                <div className="text-xs font-semibold text-gray-900 leading-none">burncloud.com</div>
                <div className="text-[10px] font-mono text-gray-500 leading-tight capitalize">{roleMeta[role].label}</div>
              </div>
            </div>
          </div>
        </header>

        {/* Page Content Viewport */}
        <main className="flex-1 overflow-y-auto p-6 md:p-8">
          <div className="max-w-7xl mx-auto space-y-6">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
