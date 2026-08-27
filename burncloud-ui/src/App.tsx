/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React from 'react';
import { BrowserRouter, Routes as RouterRoutes, Route, Navigate } from 'react-router-dom';
import { I18nProvider } from '@/i18n/I18nContext';
import { RoleProvider, useRole } from '@/context/RoleContext';
import { Layout } from '@/components/Layout';

// Public Pages
import { Home } from '@/pages/Home';
import { Login } from '@/pages/Login';
import { Register } from '@/pages/Register';

// Buyer Role Pages
import { BuyerOverview } from '@/pages/buyer/BuyerOverview';
import { BuyerPlayground } from '@/pages/buyer/BuyerPlayground';
import { BuyerMarketplace } from '@/pages/buyer/BuyerMarketplace';
import { BuyerAPIKeys } from '@/pages/buyer/BuyerAPIKeys';
import { BuyerUsage } from '@/pages/buyer/BuyerUsage';
import { BuyerBilling } from '@/pages/buyer/BuyerBilling';
import { BuyerLogs } from '@/pages/buyer/BuyerLogs';

// Supplier Role Pages
import { SupplierOverview } from '@/pages/supplier/SupplierOverview';
import { SupplierResources } from '@/pages/supplier/SupplierResources';
import { SupplierDeployments } from '@/pages/supplier/SupplierDeployments';
import { SupplierEarnings } from '@/pages/supplier/SupplierEarnings';
import { SupplierSettlements } from '@/pages/supplier/SupplierSettlements';
import { SupplierReliability } from '@/pages/supplier/SupplierReliability';
import { SupplierSettings } from '@/pages/supplier/SupplierSettings';

// Admin Role Pages
import { AdminOverview } from '@/pages/admin/AdminOverview';
import { AdminSupply } from '@/pages/admin/AdminSupply';
import { AdminCapacity } from '@/pages/admin/AdminCapacity';
import { AdminDemand } from '@/pages/admin/AdminDemand';
import { AdminModels } from '@/pages/admin/AdminModels';
import { AdminRevenue } from '@/pages/admin/AdminRevenue';
import { AdminSettlements } from '@/pages/admin/AdminSettlements';
import { AdminSuppliers } from '@/pages/admin/AdminSuppliers';
import { AdminCustomers } from '@/pages/admin/AdminCustomers';
import { AdminOperations } from '@/pages/admin/AdminOperations';
import { AdminSettings } from '@/pages/admin/AdminSettings';

// Dynamic Root Router
function DynamicRoot() {
  const { role } = useRole();
  if (role === 'supplier') {
    return <SupplierOverview />;
  }
  if (role === 'admin') {
    return <AdminOverview />;
  }
  return <BuyerOverview />;
}

export default function App() {
  return (
    <BrowserRouter>
      <I18nProvider>
        <RoleProvider>
          <Layout>
            <RouterRoutes>
              {/* Dynamic Root Route */}
              <Route path="/" element={<DynamicRoot />} />

              {/* Public Pages */}
              <Route path="/home" element={<Home />} />
              <Route path="/landing" element={<Home />} />
              <Route path="/login" element={<Login />} />
              <Route path="/register" element={<Register />} />

              {/* Buyer Routes */}
              <Route path="/buyer" element={<BuyerOverview />} />
              <Route path="/buyer/overview" element={<BuyerOverview />} />
              <Route path="/buyer/playground" element={<BuyerPlayground />} />
              <Route path="/buyer/marketplace" element={<BuyerMarketplace />} />
              <Route path="/buyer/api-keys" element={<BuyerAPIKeys />} />
              <Route path="/buyer/usage" element={<BuyerUsage />} />
              <Route path="/buyer/billing" element={<BuyerBilling />} />
              <Route path="/buyer/logs" element={<BuyerLogs />} />

              {/* Aliases for Buyer Paths */}
              <Route path="/playground" element={<BuyerPlayground />} />
              <Route path="/marketplace" element={<BuyerMarketplace />} />
              <Route path="/models" element={<BuyerMarketplace />} />
              <Route path="/keys" element={<BuyerAPIKeys />} />
              <Route path="/usage" element={<BuyerUsage />} />
              <Route path="/billing" element={<BuyerBilling />} />
              <Route path="/logs" element={<BuyerLogs />} />

              {/* Supplier Routes */}
              <Route path="/supplier" element={<SupplierOverview />} />
              <Route path="/supplier/overview" element={<SupplierOverview />} />
              <Route path="/supplier/resources" element={<SupplierResources />} />
              <Route path="/supplier/deployments" element={<SupplierDeployments />} />
              <Route path="/supplier/earnings" element={<SupplierEarnings />} />
              <Route path="/supplier/settlements" element={<SupplierSettlements />} />
              <Route path="/supplier/reliability" element={<SupplierReliability />} />
              <Route path="/supplier/settings" element={<SupplierSettings />} />

              {/* Admin Routes */}
              <Route path="/admin" element={<AdminOverview />} />
              <Route path="/admin/overview" element={<AdminOverview />} />
              <Route path="/admin/supply" element={<AdminSupply />} />
              <Route path="/admin/capacity" element={<AdminCapacity />} />
              <Route path="/admin/demand" element={<AdminDemand />} />
              <Route path="/admin/models" element={<AdminModels />} />
              <Route path="/admin/revenue" element={<AdminRevenue />} />
              <Route path="/admin/settlements" element={<AdminSettlements />} />
              <Route path="/admin/suppliers" element={<AdminSuppliers />} />
              <Route path="/admin/customers" element={<AdminCustomers />} />
              <Route path="/admin/operations" element={<AdminOperations />} />
              <Route path="/admin/settings" element={<AdminSettings />} />

              {/* Fallback */}
              <Route path="*" element={<Navigate to="/" replace />} />
            </RouterRoutes>
          </Layout>
        </RoleProvider>
      </I18nProvider>
    </BrowserRouter>
  );
}
