import React, { createContext, useContext, useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';

export type UserRole = 'buyer' | 'supplier' | 'admin';

interface RoleContextType {
  role: UserRole;
  setRole: (role: UserRole) => void;
  balance: number;
  setBalance: React.Dispatch<React.SetStateAction<number>>;
  todaySpend: number;
  supplierEarningsToday: number;
  adminRevenueToday: number;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  isSearchOpen: boolean;
  setIsSearchOpen: (open: boolean) => void;
  notificationsCount: number;
}

const RoleContext = createContext<RoleContextType | undefined>(undefined);

export const RoleProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const navigate = useNavigate();

  // Detect initial role from URL
  const getInitialRole = (): UserRole => {
    if (location.pathname.startsWith('/supplier')) return 'supplier';
    if (location.pathname.startsWith('/admin')) return 'admin';
    return 'buyer';
  };

  const [role, setRoleState] = useState<UserRole>(getInitialRole);
  const [balance, setBalance] = useState<number>(128.50);
  const [todaySpend] = useState<number>(14.28);
  const [supplierEarningsToday] = useState<number>(382.40);
  const [adminRevenueToday] = useState<number>(18450.00);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isSearchOpen, setIsSearchOpen] = useState<boolean>(false);

  // Sync role with route change if route clearly indicates a different role
  useEffect(() => {
    if (location.pathname.startsWith('/supplier') && role !== 'supplier') {
      setRoleState('supplier');
    } else if (location.pathname.startsWith('/admin') && role !== 'admin') {
      setRoleState('admin');
    } else if (location.pathname.startsWith('/buyer') && role !== 'buyer') {
      setRoleState('buyer');
    }
  }, [location.pathname]);

  const setRole = (newRole: UserRole) => {
    setRoleState(newRole);
    if (newRole === 'buyer') {
      navigate('/buyer/overview');
    } else if (newRole === 'supplier') {
      navigate('/supplier/overview');
    } else if (newRole === 'admin') {
      navigate('/admin/overview');
    }
  };

  const notificationsCount = role === 'buyer' ? 1 : role === 'supplier' ? 1 : 2;

  return (
    <RoleContext.Provider
      value={{
        role,
        setRole,
        balance,
        setBalance,
        todaySpend,
        supplierEarningsToday,
        adminRevenueToday,
        searchQuery,
        setSearchQuery,
        isSearchOpen,
        setIsSearchOpen,
        notificationsCount,
      }}
    >
      {children}
    </RoleContext.Provider>
  );
};

export const useRole = (): RoleContextType => {
  const context = useContext(RoleContext);
  if (!context) {
    throw new Error('useRole must be used within a RoleProvider');
  }
  return context;
};
