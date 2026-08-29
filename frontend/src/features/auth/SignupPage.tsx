import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useSignup } from '../../api/generated/auth/auth'
import { useRefreshCurrentUser } from './useCurrentUser'

export function SignupPage() {
  const [tenantName, setTenantName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const navigate = useNavigate()
  const refreshCurrentUser = useRefreshCurrentUser()
  const signup = useSignup()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await signup.mutateAsync({
      data: { tenant_name: tenantName, email, password },
    })
    if (result.status === 200) {
      await refreshCurrentUser()
      navigate('/', { replace: true })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="auth-page">
      <form className="auth-form" onSubmit={handleSubmit}>
        <h1>Create your tenant</h1>
        {error && <p className="form-error">{error}</p>}
        <label>
          Company / tenant name
          <input
            type="text"
            value={tenantName}
            onChange={(e) => setTenantName(e.target.value)}
            required
            autoFocus
          />
        </label>
        <label>
          Email
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </label>
        <label>
          Password
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={8}
          />
        </label>
        <button type="submit" disabled={signup.isPending}>
          {signup.isPending ? 'Creating…' : 'Sign up'}
        </button>
        <p className="auth-switch">
          Already have an account? <Link to="/login">Log in</Link>
        </p>
      </form>
    </div>
  )
}
