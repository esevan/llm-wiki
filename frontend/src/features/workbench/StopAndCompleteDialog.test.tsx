import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { StopAndCompleteDialog } from './StopAndCompleteDialog';
import { taskCompletionClient } from '../../services/taskExecutionClient';
vi.mock('../../services/taskExecutionClient',()=>({taskCompletionClient:{preview:vi.fn(),control:vi.fn()}}));
const review={taskId:'task',taskRevision:2,taskState:'in_progress',reviewHash:'review',runs:[{id:'run',instruction:'Organize the report',status:'needs_attention'}],queued:[{id:'queue',instruction:'Publish a proposed follow-up'}],responses:[{id:'approval',title:'Approve a command',kind:'command_approval'}]};
const progress={state:'stopping' as const,review,remaining:review,issues:[]};
beforeEach(()=>{
 Object.defineProperty(HTMLDialogElement.prototype,'showModal',{configurable:true,value:function(this:HTMLDialogElement){this.open=true;}});
 Object.defineProperty(HTMLDialogElement.prototype,'close',{configurable:true,value:function(this:HTMLDialogElement){this.open=false;}});
 document.documentElement.lang='en';vi.clearAllMocks();vi.mocked(taskCompletionClient.preview).mockResolvedValue(review);vi.mocked(taskCompletionClient.control).mockResolvedValue(progress);});
it('reviews affected executions, queue and responses before any stop, and cancellation before confirmation does nothing',async()=>{
 const onClose=vi.fn();render(<StopAndCompleteDialog taskId="task" revision={2} title="Finish report" evidence="Saved report" onComplete={vi.fn()} onClose={onClose}/>);
 await screen.findByText('Organize the report');expect(screen.getByText('Publish a proposed follow-up')).toBeVisible();expect(screen.getByText('Approve a command')).toBeVisible();
 expect(taskCompletionClient.control).not.toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:'Cancel completion'}));expect(onClose).toHaveBeenCalledOnce();expect(taskCompletionClient.control).not.toHaveBeenCalled();
});
it('sends one exact confirmation despite repeated clicks and waits beyond the stop acknowledgment',async()=>{
 let resolve!:(value:typeof progress)=>void;vi.mocked(taskCompletionClient.control).mockImplementationOnce(()=>new Promise(value=>{resolve=value;}));
 const onComplete=vi.fn();render(<StopAndCompleteDialog taskId="task" revision={2} title="Report" evidence="Saved report" onComplete={onComplete} onClose={vi.fn()}/>);
 await screen.findByText('Organize the report');const confirm=screen.getByRole('button',{name:'Stop and complete'});fireEvent.click(confirm);fireEvent.click(confirm);
 expect(taskCompletionClient.control).toHaveBeenCalledTimes(1);expect(taskCompletionClient.control).toHaveBeenCalledWith(expect.objectContaining({action:'begin',reviewHash:'review',expectedTaskRevision:2,evidence:'Saved report'}));
 await act(async()=>resolve(progress));expect(onComplete).not.toHaveBeenCalled();expect(screen.getByText(/Waiting for confirmed termination/)).toBeVisible();
 vi.mocked(taskCompletionClient.control).mockResolvedValueOnce({...progress,state:'completed'});fireEvent.click(screen.getByRole('button',{name:'Check again'}));await waitFor(()=>expect(onComplete).toHaveBeenCalledOnce());
});
it('lets cancellation win while a delayed begin response is still in flight',async()=>{
 let resolve!:(value:typeof progress)=>void;vi.mocked(taskCompletionClient.control).mockImplementationOnce(()=>new Promise(value=>{resolve=value;})).mockResolvedValueOnce({...progress,state:'cancelled'});
 const onComplete=vi.fn(),onClose=vi.fn();render(<StopAndCompleteDialog taskId="task" revision={2} title="Report" evidence="Saved report" onComplete={onComplete} onClose={onClose}/>);
 await screen.findByText('Organize the report');fireEvent.click(screen.getByRole('button',{name:'Stop and complete'}));fireEvent.click(screen.getByRole('button',{name:'Cancel completion'}));
 await waitFor(()=>expect(onClose).toHaveBeenCalledOnce());await act(async()=>resolve({...progress}));expect(onComplete).not.toHaveBeenCalled();
 expect(taskCompletionClient.control).toHaveBeenLastCalledWith(expect.objectContaining({action:'cancel'}));
});
it('requires a refreshed review after a stale confirmation and preserves the completion evidence',async()=>{
 vi.mocked(taskCompletionClient.control).mockRejectedValueOnce(new Error('completion_review_stale'));
 vi.mocked(taskCompletionClient.preview).mockResolvedValueOnce(review).mockResolvedValueOnce({...review,reviewHash:'updated',queued:[]});
 render(<StopAndCompleteDialog taskId="task" revision={2} title="Report" evidence="Saved report" onComplete={vi.fn()} onClose={vi.fn()}/>);
 await screen.findByText('Organize the report');fireEvent.click(screen.getByRole('button',{name:'Stop and complete'}));await screen.findByText(/Review the updated list/);
 fireEvent.click(screen.getByRole('button',{name:'Stop and complete'}));await waitFor(()=>expect(taskCompletionClient.control).toHaveBeenLastCalledWith(expect.objectContaining({action:'begin',reviewHash:'updated',evidence:'Saved report'})));
});
it('recovers an already approved pending completion without dispatching a second begin',async()=>{
 vi.mocked(taskCompletionClient.preview).mockResolvedValueOnce({...review,activeOperationId:'persisted',activeEvidence:'Original completion evidence'});
 vi.mocked(taskCompletionClient.control).mockResolvedValueOnce({...progress,issues:['Exact provider turn is unavailable']});
 render(<StopAndCompleteDialog taskId="task" revision={2} title="Report" evidence="New input" onComplete={vi.fn()} onClose={vi.fn()}/>);
 await screen.findByText('Exact provider turn is unavailable');expect(taskCompletionClient.control).toHaveBeenCalledWith({action:'poll',taskId:'task',operationId:'persisted'});expect(screen.getByText('Original completion evidence')).toBeVisible();
});
