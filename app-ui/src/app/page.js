"use client"

import { fetchIndex } from '@/server';
import { useEffect, useState } from 'react'

export default function Home() {
  const [message, setMessage] = useState(0);


  let fetchMessage = async () => {
    let ret = fetchIndex()
    setMessage(ret)
  }

  useEffect(() => {
    fetchMessage()

    return () => {
    };
  }, []);

  return (
    <main >
      The Q-Bot
      <div>
        {message}
      </div>
    </main>
  )
}
