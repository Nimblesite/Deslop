// [DOC-FILMS-DELIVERY] Import a verified factory delivery and emit its evidence.
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';

const root=path.resolve(import.meta.dirname,'../..');
const [slug,ocrFile]=process.argv.slice(2);
if(!/^[a-z-]+$/.test(slug||'') || !ocrFile) throw new Error('Usage: import-doc-film.mjs <slug> <OCR receipt>');
const videos=process.env.VIDEOS_DIR || path.resolve(root,'../Videos');
const folder=path.join(videos,'projects/deslop/videos',`docs-${slug}`);
const {default:film}=await import(pathToFileURL(path.join(folder,'film.mjs')));
const stem=`deslop-docs-${slug}`;
const dist=path.join(folder,'dist');
const read=(file)=>JSON.parse(fs.readFileSync(file,'utf8'));
const timeline=read(path.join(dist,`${stem}.timeline.json`));
const expected=film.script.lines;
if(timeline.lines.length!==expected.length || expected.some((line,i)=>line.text!==timeline.lines[i].text || line.scene!==timeline.lines[i].scene)) throw new Error('Delivered narration is stale');
const media=path.join(dist,`${stem}.mp4`);
const probe=JSON.parse(execFileSync('ffprobe',['-v','error','-show_streams','-show_format','-of','json',media],{encoding:'utf8'}));
const video=probe.streams.find(s=>s.codec_type==='video');
const audio=probe.streams.find(s=>s.codec_type==='audio');
const seconds=Number(probe.format.duration);
if(video?.codec_name!=='h264'||audio?.codec_name!=='aac'||video.width!==1920||video.height!==1080) throw new Error('Unexpected delivery encoding');
if(seconds<film.durationBounds[0]||seconds>film.durationBounds[1]) throw new Error('Duration outside film bounds');
const checks=read(path.resolve(ocrFile));
if(!checks.expected?.length || !Number.isFinite(checks.seconds)) throw new Error('Missing OCR check specification');
function checkFrame(seconds,expected) {
  return JSON.parse(execFileSync('python3',['-m','factory.verify','--video',media,'--at',String(seconds),...expected.flatMap(text=>['--expect',text])],{cwd:videos,encoding:'utf8'}));
}
const headingText=text=>text.toLowerCase().replace(/\s+/g,' ').trim();
const sceneChecks=timeline.scenes.map(scene=>{
  const reader='import json,sys; from pathlib import Path; from factory.verify import read_video_text; print(json.dumps(read_video_text(Path(sys.argv[1]),float(sys.argv[2]),[])))';
  const result=JSON.parse(execFileSync('python3',['-c',reader,media,String((scene.start+scene.end)/2)],{cwd:videos,encoding:'utf8'}));
  if(!headingText(result.text).includes(headingText(scene.title))) throw new Error(`Missing rendered chapter title: ${scene.title}`);
  return result;
});
const ocr=checkFrame(checks.seconds,checks.expected);
const suffixes=['.mp4','-poster.jpg','.en.vtt','.chapters.vtt'];
for(const suffix of suffixes) if(!fs.statSync(path.join(dist,stem+suffix)).size) throw new Error(`Empty ${suffix}`);
const destination=path.join(root,'site/src/assets/video/docs');
fs.mkdirSync(destination,{recursive:true});
for(const suffix of suffixes) fs.copyFileSync(path.join(dist,stem+suffix),path.join(destination,stem+suffix));
const dataPath=path.join(root,'site/src/_data/docFilms.json');
const data=read(dataPath);
const rounded=Math.round(seconds);
data[slug]={title:film.scenes[0].title,duration:`${Math.floor(rounded/60)}:${String(rounded%60).padStart(2,'0')}`};
fs.writeFileSync(dataPath,JSON.stringify(data,null,2)+'\n');
const digest=createHash('sha256').update(fs.readFileSync(media)).digest('hex');
const report=path.join(root,'reports/videos',`${slug}-media.md`);
fs.mkdirSync(path.dirname(report),{recursive:true});
fs.writeFileSync(report,`# ${data[slug].title} media verification\n\n- Duration: ${seconds} seconds\n- Video: ${video.codec_name}, ${video.width} x ${video.height}\n- Audio: ${audio.codec_name}\n- Narration: ${expected.length} lines match the current film script\n- Delivery: MP4, poster, English captions and chapters present\n- Scene coverage: ${sceneChecks.length} of ${timeline.scenes.length} rendered chapter titles verified\n- OCR at ${ocr.seconds}s: ${ocr.expected.join(', ')}\n- SHA-256: ${digest}\n\nBrowser playback and visual review are separate checks.\n`);
process.stdout.write(`Imported ${slug}: ${seconds}s; ${report}\n`);
