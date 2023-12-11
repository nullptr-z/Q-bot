"use client"

import axios from "axios"

export async function fetchIndex() {
  let ret = await axios.get("/")
  console.log("【 ret 】==>", ret);
  return "the index"
}
