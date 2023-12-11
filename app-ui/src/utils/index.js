// utils/api.js
import axios from 'axios';

export function initAxios() {
  axios.defaults.baseURL = 'https://qbot.ai:8080';
  axios.defaults.headers = { 'Content-Type': 'application/json' }
}
