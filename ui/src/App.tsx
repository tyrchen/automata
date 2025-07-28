import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { useAppStore } from './stores';
import AppLayout from './components/layout/AppLayout';
import Dashboard from './pages/Dashboard';
import WorkflowList from './pages/WorkflowList';
import WorkflowEditor from './pages/WorkflowEditor';
import WorkflowDetail from './pages/WorkflowDetail';
import ExecutionList from './pages/ExecutionList';
import ExecutionDetail from './pages/ExecutionDetail';

function App() {
  const { isDarkMode } = useAppStore();

  useEffect(() => {
    // Apply dark mode class to document
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [isDarkMode]);

  return (
    <Router>
      <div className={`min-h-screen ${isDarkMode ? 'dark' : ''}`}>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<Dashboard />} />
            <Route path="workflows" element={<WorkflowList />} />
            <Route path="workflows/new" element={<WorkflowEditor />} />
            <Route path="workflows/:id" element={<WorkflowDetail />} />
            <Route path="workflows/:id/edit" element={<WorkflowEditor />} />
            <Route path="executions" element={<ExecutionList />} />
            <Route path="executions/:id" element={<ExecutionDetail />} />
            <Route path="settings" element={<div className="p-6"><h1>Settings (Coming Soon)</h1></div>} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </div>
    </Router>
  );
}

export default App;
