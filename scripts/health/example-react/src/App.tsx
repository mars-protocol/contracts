import { useState } from 'react'
import { creditManager } from '../../../deploy/addresses/osmo-test-4.json'
import { useQuery } from 'react-query'
import { fetchHealth, fetchPositions } from './utils'
import ReactJson from 'react-json-view'
import { SyncLoader } from 'react-spinners'

function App() {
  const [cmAddr, setCmAddr] = useState(creditManager)
  const [accountId, setAccountId] = useState('1')

  const postions = useQuery([cmAddr, accountId, 'positions'], () =>
    fetchPositions(cmAddr, accountId),
  )

  const health = useQuery([cmAddr, accountId, 'health'], () => fetchHealth(cmAddr, accountId))

  return (
    <main>
      <div>
        <label>
          Credit Manager contract addr:
          <input
            type='text'
            value={cmAddr}
            onChange={({ target: { value } }) => setCmAddr(value)}
          />
        </label>
      </div>
      <div>
        <label>
          Credit account:
          <input
            type='text'
            value={accountId}
            onChange={({ target: { value } }) => setAccountId(value)}
          />
        </label>
      </div>
      <div>
        Positions:{' '}
        {postions.isLoading ? (
          <SyncLoader color='#36d7b7' />
        ) : postions.error ? (
          'error ❌'
        ) : postions.data ? (
          <ReactJson src={postions.data} />
        ) : undefined}
      </div>
      <div>
        Health:{' '}
        {health.isLoading ? (
          <SyncLoader color='#36d7b7' />
        ) : health.error ? (
          'error ❌'
        ) : health.data ? (
          <ReactJson src={health.data} />
        ) : undefined}
      </div>
    </main>
  )
}

export default App
