import axios from "axios"

export async function fetchIndex() {
  let ret = await axios.get("http://127.0.0.1:8080/")
  console.log("【 ret 】==>", ret);
  return "the index"
}
