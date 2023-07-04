// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { JsonRpcProvider, SuiAddress, SuiObjectResponse } from '@mysten/sui.js';
import { KIOSK_OWNER_CAP, KioskData, KioskItem, fetchKiosk, getOwnedKiosks } from '@mysten/kiosk';
import { useQuery } from '@tanstack/react-query';
import { useRpcClient } from '../api/RpcClientContext';

const getKioskId = (obj: SuiObjectResponse) =>
	obj.data?.content &&
	'fields' in obj.data.content &&
	(obj.data.content.fields.for ?? obj.data.content.fields.kiosk);

// OriginByte module for mainnet (we only support mainnet)
export const ORIGINBYTE_KIOSK_MODULE =
	'0x95a441d389b07437d00dd07e0b6f05f513d7659b13fd7c5d3923c7d9d847199b::ob_kiosk' as const;
export const ORIGINBYTE_KIOSK_OWNER_TOKEN = `${ORIGINBYTE_KIOSK_MODULE}::OwnerToken`;

export function isKioskOwnerToken(object?: SuiObjectResponse) {
	if (!object) return false;
	return [KIOSK_OWNER_CAP, ORIGINBYTE_KIOSK_OWNER_TOKEN].includes(object.data?.type ?? '');
}

async function getOriginByteKioskContents(address: SuiAddress, rpc: JsonRpcProvider) {
	const data = await rpc.getOwnedObjects({
		owner: address,
		filter: {
			StructType: ORIGINBYTE_KIOSK_OWNER_TOKEN,
		},
		options: {
			showContent: true,
		},
	});

	const ids = data.data.map((object) => getKioskId(object) ?? []);

	const kiosks = new Map<string, { items: SuiObjectResponse[] }>();

	// fetch the user's kiosks
	const ownedKiosks = await rpc.multiGetObjects({
		ids: ids.flat(),
		options: {
			showContent: true,
		},
	});

	// find object IDs within a kiosk
	await Promise.all(
		ownedKiosks.map(async (kiosk) => {
			if (!kiosk.data?.objectId) return [];
			const objects = await rpc.getDynamicFields({
				parentId: kiosk.data.objectId,
			});

			const objectIds = objects.data.map((obj) => obj.objectId);

			// fetch the contents of the objects within a kiosk
			const kioskContent = await rpc.multiGetObjects({
				ids: objectIds,
				options: {
					showDisplay: true,
					showType: true,
				},
			});

			kiosks.set(kiosk.data.objectId, { items: kioskContent });
		}),
	);

	return kiosks;
}

type KioskContents = Omit<KioskData, 'items'> & {
	items: Partial<KioskItem & SuiObjectResponse>[];
};

async function getSuiKioskContents(address: SuiAddress, rpc: JsonRpcProvider) {
	const ownedKiosks = await getOwnedKiosks(rpc, address!);

	const kiosks = new Map<string, KioskContents>();

	await Promise.all(
		ownedKiosks.kioskIds.map(async (id) => {
			const kiosk = await fetchKiosk(rpc, id, { limit: 1000 }, {});
			const contents = await rpc.multiGetObjects({
				ids: kiosk.data.itemIds,
				options: { showDisplay: true, showContent: true, showOwner: true },
			});

			const items = contents.map((object) => {
				const kioskData = kiosk.data.items.find((item) => item.objectId === object.data?.objectId);
				return { ...object, ...kioskData };
			});

			kiosks.set(id, { ...kiosk.data, items });
		}, kiosks),
	);

	return kiosks;
}

export function useGetKioskContents(address?: SuiAddress | null, disableOriginByteKiosk?: boolean) {
	const rpc = useRpcClient();
	return useQuery({
		// eslint-disable-next-line @tanstack/query/exhaustive-deps
		queryKey: ['get-kiosk-contents', address, disableOriginByteKiosk],
		queryFn: async () => {
			const suiKiosks = await getSuiKioskContents(address!, rpc);
			const obKioskContents = await getOriginByteKioskContents(address!, rpc);
			const list = [
				...Array.from((suiKiosks ?? []).values()),
				...Array.from(obKioskContents.values() ?? []),
			].flatMap((d) => d.items);
			return {
				list,
				kiosks: {
					sui: suiKiosks,
					originByte: obKioskContents,
				},
			};
		},
	});
}
