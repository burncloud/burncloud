import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { Users, UserCheck, UserX, Mail, Plus, Search, ShieldCheck, Trash2, ArrowRight, CheckCircle2, AlertCircle } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

interface TeamMember {
  id: string;
  name: string;
  email: string;
  role: 'Owner' | 'Admin' | 'Developer' | 'Engineer' | 'Viewer';
  status: 'Active' | 'Pending Invite' | 'Suspended';
  addedDate: string;
}

const INITIAL_TEAM: TeamMember[] = [
  { id: 't1', name: 'William Hayes', email: 'william@burncloud.com', role: 'Owner', status: 'Active', addedDate: 'Jul 12, 2025' },
  { id: 't2', name: 'Sarah Chen', email: 'sarah.chen@burncloud.com', role: 'Admin', status: 'Active', addedDate: 'Nov 24, 2025' },
  { id: 't3', name: 'Aris Thorne', email: 'aris.thorne@burncloud.com', role: 'Engineer', status: 'Active', addedDate: 'Feb 15, 2026' },
  { id: 't4', name: 'Elena Rostova', email: 'elena.rostova@burncloud.com', role: 'Viewer', status: 'Active', addedDate: 'May 02, 2026' },
  { id: 't5', name: 'James Carter', email: 'james.carter@burncloud.com', role: 'Developer', status: 'Pending Invite', addedDate: 'Jul 08, 2026' },
];

export function Team() {
  const [team, setTeam] = useState<TeamMember[]>(INITIAL_TEAM);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedMember, setSelectedMember] = useState<TeamMember | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isEditMode, setIsEditMode] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [showToast, setShowToast] = useState(false);

  // Form states
  const [formName, setFormName] = useState('');
  const [formEmail, setFormEmail] = useState('');
  const [formRole, setFormRole] = useState<'Owner' | 'Admin' | 'Developer' | 'Engineer' | 'Viewer'>('Developer');

  const filteredTeam = team.filter(m => 
    m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    m.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
    m.role.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleOpenInvite = () => {
    setIsEditMode(false);
    setFormName('');
    setFormEmail('');
    setFormRole('Developer');
    setIsDrawerOpen(true);
  };

  const handleOpenEdit = (m: TeamMember) => {
    // Cannot edit owner easily in mock
    if (m.role === 'Owner') return;
    setSelectedMember(m);
    setIsEditMode(true);
    setFormName(m.name);
    setFormEmail(m.email);
    setFormRole(m.role);
    setIsDrawerOpen(true);
  };

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formEmail) return;

    setIsSaving(true);
    setTimeout(() => {
      if (isEditMode && selectedMember) {
        setTeam(prev => prev.map(m => m.id === selectedMember.id ? {
          ...m,
          name: formName || m.name,
          email: formEmail,
          role: formRole
        } : m));
      } else {
        const newMember: TeamMember = {
          id: 't_' + Date.now(),
          name: formName || formEmail.split('@')[0],
          email: formEmail,
          role: formRole,
          status: 'Pending Invite',
          addedDate: new Date().toLocaleDateString('en-US', { month: 'short', day: '2-digit', year: 'numeric' })
        };
        setTeam(prev => [...prev, newMember]);
      }
      setIsSaving(false);
      setIsDrawerOpen(false);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 3000);
    }, 1000);
  };

  const handleRevoke = (id: string) => {
    if (confirm('Are you sure you want to revoke access for this team member?')) {
      setTeam(prev => prev.filter(m => m.id !== id));
      setIsDrawerOpen(false);
    }
  };

  const handleToggleSuspend = (id: string, currentStatus: string) => {
    setTeam(prev => prev.map(m => {
      if (m.id === id) {
        return {
          ...m,
          status: currentStatus === 'Suspended' ? 'Active' : 'Suspended'
        };
      }
      return m;
    }));
  };

  const activeCount = team.filter(m => m.status === 'Active').length;
  const pendingCount = team.filter(m => m.status === 'Pending Invite').length;

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out relative">
      {/* Toast Notification */}
      <AnimatePresence>
        {showToast && (
          <motion.div 
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="fixed top-6 right-6 z-50 bg-gray-900 text-white px-4 py-3 rounded-xl shadow-lg flex items-center gap-2.5 text-xs font-semibold"
          >
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            {isEditMode ? 'Team member updated.' : 'Invitation dispatch complete.'}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Team Management</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Control organization member accounts, edit administrative permissions, and audit secure platform seats.</p>
        </div>
        <Button onClick={handleOpenInvite} className="gap-2 text-[13px] self-start sm:self-auto">
          <Plus className="w-4 h-4" /> Invite Organization Member
        </Button>
      </div>

      {/* KPIs */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-5">
        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Total Seats Allocated</span>
            <p className="text-2xl font-bold text-gray-900">{team.length}</p>
          </div>
          <div className="w-10 h-10 bg-gray-50 rounded-xl flex items-center justify-center border border-gray-100">
            <Users className="w-5 h-5 text-gray-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Active Members</span>
            <p className="text-2xl font-bold text-emerald-600">{activeCount}</p>
          </div>
          <div className="w-10 h-10 bg-emerald-50 rounded-xl flex items-center justify-center border border-emerald-100">
            <UserCheck className="w-5 h-5 text-emerald-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Pending Invitations</span>
            <p className="text-2xl font-bold text-amber-600">{pendingCount}</p>
          </div>
          <div className="w-10 h-10 bg-amber-50 rounded-xl flex items-center justify-center border border-amber-100">
            <Mail className="w-5 h-5 text-amber-600" />
          </div>
        </Card>

        <Card className="p-5 flex items-center justify-between">
          <div className="space-y-1">
            <span className="text-[12px] font-semibold text-gray-400 uppercase tracking-wider">Role Access Standard</span>
            <p className="text-2xl font-bold text-blue-600">RBAC Enabled</p>
          </div>
          <div className="w-10 h-10 bg-blue-50 rounded-xl flex items-center justify-center border border-blue-100">
            <ShieldCheck className="w-5 h-5 text-blue-600" />
          </div>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Members Table */}
        <div className="lg:col-span-2">
          <Card className="overflow-hidden">
            <div className="p-5 border-b border-gray-100 flex items-center justify-between">
              <div className="relative w-full max-w-sm">
                <Search className="w-[15px] h-[15px] absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                <input 
                  type="text" 
                  placeholder="Search members by name or role..." 
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full h-9 bg-gray-50/80 border border-gray-200/80 rounded-[10px] pl-9 pr-4 text-[13px] focus:bg-white focus:ring-1 focus:ring-gray-300 focus:border-gray-300 focus:outline-none transition-all placeholder:text-gray-400"
                />
              </div>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-sm text-left">
                <thead className="text-[13px] text-gray-500 bg-gray-50/30 border-b border-gray-200/60">
                  <tr>
                    <th className="px-6 py-3.5 font-medium">User Profile</th>
                    <th className="px-6 py-3.5 font-medium">Role Type</th>
                    <th className="px-6 py-3.5 font-medium">Invitation Date</th>
                    <th className="px-6 py-3.5 font-medium text-center">Security Status</th>
                    <th className="px-6 py-3.5 font-medium"></th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {filteredTeam.map((member) => (
                    <tr 
                      key={member.id} 
                      onClick={() => member.role !== 'Owner' && handleOpenEdit(member)}
                      className={cn(
                        "transition-colors group",
                        member.role === 'Owner' ? "" : "hover:bg-gray-50/80 cursor-pointer"
                      )}
                    >
                      <td className="px-6 py-4">
                        <div className="flex items-center gap-3">
                          <div className="w-8 h-8 rounded-full bg-gray-100 text-gray-700 font-semibold text-xs flex items-center justify-center border border-gray-250">
                            {member.name.split(' ').map(n => n[0]).join('')}
                          </div>
                          <div>
                            <span className="font-semibold text-sm text-gray-900 block">{member.name}</span>
                            <span className="text-xs text-gray-400">{member.email}</span>
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-4">
                        <Badge variant={
                          member.role === 'Owner' ? 'brand' :
                          member.role === 'Admin' ? 'warning' : 'neutral'
                        }>
                          {member.role}
                        </Badge>
                      </td>
                      <td className="px-6 py-4 text-gray-500 text-[13px] tabular-nums">
                        {member.addedDate}
                      </td>
                      <td className="px-6 py-4 text-center">
                        <Badge variant={
                          member.status === 'Active' ? 'success' :
                          member.status === 'Pending Invite' ? 'warning' : 'neutral'
                        }>
                          {member.status}
                        </Badge>
                      </td>
                      <td className="px-6 py-4 text-right">
                        {member.role !== 'Owner' && (
                          <Button variant="ghost" size="sm" className="opacity-0 group-hover:opacity-100 transition-all">
                            Manage
                          </Button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>

        {/* Roles Details Card */}
        <div className="space-y-4">
          <h3 className="text-base font-semibold text-gray-900 tracking-tight">Role Hierarchy Guidelines</h3>
          <Card className="p-5 space-y-4 border border-gray-200/80">
            {[
              { role: 'Owner', desc: 'Complete access controls, billing administration, key provisioning, and member termination privileges.' },
              { role: 'Admin', desc: 'Manage all routes, modify key environments, invite and manage team members, and configure guardrails.' },
              { role: 'Engineer', desc: 'Access logs, create routing configurations, audit evaluation metrics, and test active playgrounds.' },
              { role: 'Developer', desc: 'Can generate tenant API keys, view route latency stats, and test sandbox integration endpoints.' },
              { role: 'Viewer', desc: 'Read-only access to routing health stats, cost charts, logs, and billing invoices.' }
            ].map((r) => (
              <div key={r.role} className="space-y-1">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-bold text-gray-900">{r.role}</span>
                  <div className="h-px bg-gray-100 flex-1"></div>
                </div>
                <p className="text-xs text-gray-500 leading-normal">{r.desc}</p>
              </div>
            ))}
          </Card>
        </div>
      </div>

      {/* Drawer */}
      <Drawer 
        isOpen={isDrawerOpen} 
        onClose={() => !isSaving && setIsDrawerOpen(false)} 
        title={isEditMode ? `Manage Member: ${selectedMember?.name}` : "Invite Organization Member"}
      >
        <form onSubmit={handleSave} className="p-6 space-y-6 relative">
          {isSaving && (
            <div className="absolute inset-0 bg-white/70 backdrop-blur-[2px] z-10 flex flex-col items-center justify-center">
              <div className="w-8 h-8 border-2 border-gray-200 border-t-gray-900 rounded-full animate-spin mb-3"></div>
              <p className="text-xs font-semibold text-gray-900">Synchronizing team credentials...</p>
            </div>
          )}

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-600 block">Full Name</label>
            <Input 
              required
              type="text" 
              placeholder="e.g. Elena Rostova" 
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-600 block">Email Address</label>
            <Input 
              required
              type="email" 
              placeholder="e.g. elena@burncloud.com" 
              value={formEmail}
              onChange={(e) => setFormEmail(e.target.value)}
              disabled={isEditMode}
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-600 block">Permissions Role</label>
            <select 
              value={formRole}
              onChange={(e) => setFormRole(e.target.value as any)}
              className="w-full h-10 rounded-xl border border-gray-200/80 bg-white/50 px-3 text-[13px] focus:outline-none focus:ring-4 focus:ring-gray-900/5 focus:border-gray-900/20 focus:bg-white transition-all shadow-[0_1px_2px_0_rgba(0,0,0,0.02)]"
            >
              <option value="Admin">Admin (Full write capabilities)</option>
              <option value="Engineer">Engineer (Configure routes/models)</option>
              <option value="Developer">Developer (Read/Write keys & playground)</option>
              <option value="Viewer">Viewer (Read health/cost summaries)</option>
            </select>
          </div>

          {isEditMode && selectedMember && (
            <div className="pt-4 border-t border-gray-100 space-y-3">
              <label className="text-xs font-semibold text-gray-600 block">Administrative Operations</label>
              <div className="flex gap-3">
                <Button 
                  type="button" 
                  variant="secondary" 
                  className="flex-1 text-xs gap-1.5"
                  onClick={() => handleToggleSuspend(selectedMember.id, selectedMember.status)}
                >
                  {selectedMember.status === 'Suspended' ? 'Unsuspend Account' : 'Suspend Account'}
                </Button>
                <Button 
                  type="button" 
                  variant="danger" 
                  className="flex-1 text-xs gap-1.5"
                  onClick={() => handleRevoke(selectedMember.id)}
                >
                  <UserX className="w-4 h-4" /> Revoke Seat
                </Button>
              </div>
            </div>
          )}

          <div className="pt-4 border-t border-gray-100 flex items-center justify-end gap-3">
            <Button type="button" variant="secondary" onClick={() => setIsDrawerOpen(false)} disabled={isSaving}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSaving}>
              {isEditMode ? 'Update Permissions' : 'Send Invitation'}
            </Button>
          </div>
        </form>
      </Drawer>
    </div>
  );
}
