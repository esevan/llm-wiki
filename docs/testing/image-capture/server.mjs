import {createServer} from 'vite';
import react from '@vitejs/plugin-react';
const server=await createServer({configFile:false,root:'docs/testing/image-capture',publicDir:process.cwd()+'/dist',plugins:[react()],server:{host:'127.0.0.1',port:5193,strictPort:true,fs:{allow:[process.cwd()]}}});
await server.listen(); console.log('Synthetic full App: http://127.0.0.1:5193');
