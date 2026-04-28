"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { MessageSquare, CheckCircle2, AlertCircle, Clock, User } from "lucide-react";
import { format } from "date-fns";

interface ReviewTimelineProps {
  reviewId: string;
}

export default function ReviewTimeline({ reviewId }: ReviewTimelineProps) {
  const queryClient = useQueryClient();
  const { data: details, isLoading } = useQuery({
    queryKey: ["collaborative-review", reviewId],
    queryFn: () => api.getCollaborativeReview(reviewId),
  });

  if (isLoading) return <div className="animate-pulse space-y-4"><div className="h-20 bg-muted rounded-xl" /></div>;
  if (!details) return null;

  const { review, reviewers, comments } = details;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold flex items-center gap-2">
          <Clock className="w-5 h-5 text-primary" />
          Review Timeline
        </h3>
        <span className={`px-3 py-1 rounded-full text-xs font-bold uppercase ${
          review.status === 'approved' ? 'bg-green-500/10 text-green-500' :
          review.status === 'changes_requested' ? 'bg-red-500/10 text-red-500' :
          'bg-amber-500/10 text-amber-500'
        }`}>
          {review.status.replace('_', ' ')}
        </span>
      </div>

      <div className="space-y-8 relative before:absolute before:left-4 before:top-2 before:bottom-2 before:w-0.5 before:bg-border">
        {comments.map((comment: any) => (
          <div key={comment.id} className="relative pl-10">
            <div className="absolute left-0 top-1 w-8 h-8 rounded-full bg-card border border-border flex items-center justify-center z-10">
              <MessageSquare className="w-4 h-4 text-muted-foreground" />
            </div>
            <div className="bg-card border border-border rounded-xl p-4 shadow-sm">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-semibold flex items-center gap-1">
                  <User className="w-3 h-3" /> {comment.user_id.slice(0, 8)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {format(new Date(comment.created_at), "MMM d, h:mm a")}
                </span>
              </div>
              <p className="text-sm text-foreground mb-2">{comment.content}</p>
              {(comment.line_number || comment.abi_path) && (
                <div className="text-[10px] uppercase tracking-wider font-bold text-primary bg-primary/5 px-2 py-0.5 rounded w-fit">
                  {comment.abi_path ? `ABI: ${comment.abi_path}` : `Line ${comment.line_number}`}
                </div>
              )}
            </div>
          </div>
        ))}

        {reviewers.map((reviewer: any) => (
          reviewer.status !== 'pending' && (
            <div key={reviewer.id} className="relative pl-10">
              <div className={`absolute left-0 top-1 w-8 h-8 rounded-full border flex items-center justify-center z-10 ${
                reviewer.status === 'approved' ? 'bg-green-500/10 border-green-500/50' : 'bg-red-500/10 border-red-500/50'
              }`}>
                {reviewer.status === 'approved' ? <CheckCircle2 className="w-4 h-4 text-green-500" /> : <AlertCircle className="w-4 h-4 text-red-500" />}
              </div>
              <div className="opacity-80">
                <p className="text-sm font-medium">
                  {reviewer.user_id.slice(0, 8)} marked as <span className={reviewer.status === 'approved' ? 'text-green-500' : 'text-red-500'}>{reviewer.status.replace('_', ' ')}</span>
                </p>
                <p className="text-xs text-muted-foreground">
                  {format(new Date(reviewer.updated_at), "MMM d, h:mm a")}
                </p>
              </div>
            </div>
          )
        ))}
      </div>
    </div>
  );
}
