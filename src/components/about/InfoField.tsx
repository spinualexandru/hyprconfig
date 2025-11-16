interface InfoFieldProps {
	label: string;
	value: string | string[];
	className?: string;
}

export function InfoField({ label, value, className = "" }: InfoFieldProps) {
	const isArray = Array.isArray(value);

	return (
		<div className={className}>
			<p className="text-sm font-medium">{label}</p>
			{isArray ? (
				<div className="space-y-1">
					{value.map((item, index) => (
						<p key={index} className="text-sm text-muted-foreground">
							{item}
						</p>
					))}
				</div>
			) : (
				<p className="text-sm text-muted-foreground">{value}</p>
			)}
		</div>
	);
}
